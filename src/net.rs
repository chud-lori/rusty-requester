//! Network layer — HTTP request execution, URL normalization, client
//! construction, and error formatting. Kept free of UI concerns so it
//! can be unit-tested independently of egui.

use crate::io::curl;
use crate::model::{AppSettings, Auth, BodyExt, Environment, HttpMethod, Request, ResponseData};
use crate::sse::{self, SseEvent, SseParser};
use crate::widgets::{substitute_kvs, substitute_vars};
use base64::Engine;
use std::time::Duration;

/// One update from the send task. `Progress` is emitted while
/// streaming an SSE response — each time new events arrive the
/// task sends a fresh snapshot plus the newly-parsed events (so the
/// UI can accumulate them into a structured Events view, separate
/// from the Raw text body). `Final` is the terminal message;
/// extractors/assertions/history only run on `Final`, and
/// `is_loading` clears there.
pub enum RequestUpdate {
    Progress {
        snapshot: ResponseData,
        new_events: Vec<SseEvent>,
    },
    Final(ResponseData),
}

/// Build a `reqwest::Client` from the current app settings. Called at
/// startup and whenever the user tweaks the Settings modal — never per
/// request.
pub fn build_client(settings: &AppSettings) -> reqwest::Client {
    let mut b = reqwest::Client::builder();
    if settings.timeout_sec > 0 {
        b = b.timeout(Duration::from_secs(settings.timeout_sec));
    }
    if !settings.verify_tls {
        b = b.danger_accept_invalid_certs(true);
    }
    let proxy = settings.proxy_url.trim();
    if !proxy.is_empty() {
        if let Ok(p) = reqwest::Proxy::all(proxy) {
            b = b.proxy(p);
        }
    }
    b.build().unwrap_or_else(|_| reqwest::Client::new())
}

/// Extract `(host, path)` from an absolute URL for cookie-jar
/// matching. Never fails — returns empty strings on a malformed URL
/// so callers keep working. We avoid pulling in the `url` crate
/// since everything else we do here is string-based already.
pub fn parse_url_host_path(url: &str) -> (String, String) {
    let rest = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("ws://")
        .trim_start_matches("wss://");
    let (host_port, path_and_q) = match rest.find('/') {
        Some(i) => rest.split_at(i),
        None => (rest, "/"),
    };
    let host = host_port
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let path = path_and_q
        .split(['?', '#'])
        .next()
        .unwrap_or("/")
        .to_string();
    let path = if path.is_empty() {
        "/".to_string()
    } else {
        path
    };
    (host, path)
}

/// Ensure the URL has a scheme so reqwest can parse it. Defaults to
/// `http://` — matches curl's historical default. An API client gets
/// pointed at too many HTTP-only dev servers, reverse proxies, and
/// custom hostnames for a "smart" https default to be reliable; users
/// hitting HTTPS services can type the scheme explicitly.
pub fn ensure_url_scheme(url: &str) -> String {
    let t = url.trim();
    if t.is_empty() {
        return t.to_string();
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ws://")
        || lower.starts_with("wss://")
    {
        return t.to_string();
    }
    format!("http://{}", t)
}

/// Build a long-lived tokio runtime used to drive every HTTP request.
/// Stored on `ApiClient` and shared across `execute_request` calls so
/// we don't pay the spawn-runtime cost (~1ms + thread allocation)
/// per click of Send. Use `multi_thread` with 2 worker threads — one
/// driver, one for body streaming — small enough to keep memory low,
/// big enough to never block.
pub fn build_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .thread_name("rr-net")
        .build()
        .expect("failed to build tokio runtime")
}

/// Flatten a reqwest error's cause chain into a readable multi-line
/// string. reqwest's top-level `Display` often just says "builder
/// error" or "error sending request"; the actual reason (invalid URL,
/// DNS failure, etc.) is hidden in `source()`.
pub fn format_request_error(err: &reqwest::Error) -> String {
    use std::error::Error;
    let mut msg = format!("Error: {}", err);
    let mut source: Option<&(dyn Error + 'static)> = err.source();
    while let Some(s) = source {
        msg.push_str(&format!("\n  caused by: {}", s));
        source = s.source();
    }
    msg
}

/// Fire a single HTTP request and return a fully-populated
/// `ResponseData`. Async because `ApiClient` now spawns it onto the
/// long-lived tokio runtime as a `JoinHandle` that can be `.abort()`ed
/// mid-flight when the user clicks Cancel. Dropping the future mid-
/// `.await` cancels the underlying reqwest/hyper connection and frees
/// the resources, so Cancel is immediate (no per-chunk polling).
///
/// `client` is built once from `AppSettings` (see `build_client`) and
/// shared across calls. `max_body_bytes` is the soft cap after which
/// the body is truncated with a banner; `0` disables the cap.
pub async fn execute_request_async(
    client: reqwest::Client,
    request: Request,
    env: Option<Environment>,
    max_body_bytes: usize,
    progress: Option<std::sync::mpsc::Sender<RequestUpdate>>,
) -> ResponseData {
    let env = env.as_ref();
    let request = &request;
    let t_prepare_start = std::time::Instant::now();

    // Apply environment substitution to all string fields up-front.
    let final_url_base = substitute_vars(&request.url, env);
    let sub_params = substitute_kvs(&request.query_params, env);
    let sub_headers = substitute_kvs(&request.headers, env);
    let sub_cookies = substitute_kvs(&request.cookies, env);
    let sub_body = substitute_vars(&request.body, env);
    let sub_auth = match &request.auth {
        Auth::None => Auth::None,
        Auth::Bearer { token } => Auth::Bearer {
            token: substitute_vars(token, env),
        },
        Auth::Basic { username, password } => Auth::Basic {
            username: substitute_vars(username, env),
            password: substitute_vars(password, env),
        },
        // OAuth2 passes through unchanged — the access token comes
        // from the stored flow state, not from env-var substitution.
        // (Users who want env-var tokens should use Bearer auth; OAuth
        // is for the full authorize-redirect flow.)
        Auth::OAuth2(s) => Auth::OAuth2(s.clone()),
    };

    // Inner block is the former `runtime_handle.block_on(async { … })`
    // — same body, just unwrapped. The outer fn is async, so the
    // caller chooses how to execute (tokio::spawn with JoinHandle for
    // cancel, or block_on if cancellation isn't needed).
    {
        let final_url_base = ensure_url_scheme(&final_url_base);
        let final_url = curl::build_full_url(&final_url_base, &sub_params);

        let mut req_builder = match request.method {
            HttpMethod::GET => client.get(&final_url),
            HttpMethod::POST => client.post(&final_url),
            HttpMethod::PUT => client.put(&final_url),
            HttpMethod::DELETE => client.delete(&final_url),
            HttpMethod::PATCH => client.patch(&final_url),
            HttpMethod::HEAD => client.head(&final_url),
            HttpMethod::OPTIONS => client.request(reqwest::Method::OPTIONS, &final_url),
        };

        // Build the outgoing `Cookie` header from three sources,
        // deduped by name (last wins): the request's explicit Cookies
        // tab, explicit `Cookie:` headers, and the active
        // environment's persisted cookie jar (matched by host+path).
        let (url_host, url_path) = parse_url_host_path(&final_url);
        let jar_cookies = env
            .map(|e| crate::cookies::cookies_for_url(&e.cookies, &url_host, &url_path))
            .unwrap_or_default();
        let mut cookie_map: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        for (k, v) in jar_cookies {
            cookie_map.insert(k, v);
        }
        for c in &sub_cookies {
            if c.enabled && !c.key.is_empty() {
                cookie_map.insert(c.key.clone(), c.value.clone());
            }
        }
        for h in &sub_headers {
            if !h.enabled || h.key.trim().is_empty() {
                continue;
            }
            if h.key.eq_ignore_ascii_case("cookie") {
                // Parse "a=1; b=2" and merge into the map.
                for pair in h.value.split(';') {
                    if let Some((k, v)) = pair.split_once('=') {
                        cookie_map.insert(k.trim().to_string(), v.trim().to_string());
                    }
                }
                continue;
            }
            req_builder = req_builder.header(&h.key, &h.value);
        }
        if !cookie_map.is_empty() {
            let joined = cookie_map
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            req_builder = req_builder.header("Cookie", joined);
        }

        match &sub_auth {
            Auth::Bearer { token } if !token.is_empty() => {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
            }
            Auth::OAuth2(s) if !s.access_token.is_empty() => {
                // Send the cached access token as a Bearer header.
                // Expiry is checked by the UI (shows a warning pill
                // in the Auth tab) so the user can re-run the flow
                // before the send; this layer trusts the token it's
                // handed and lets the provider reject it if stale.
                req_builder =
                    req_builder.header("Authorization", format!("Bearer {}", s.access_token));
            }
            Auth::Basic { username, password } if !username.is_empty() => {
                let encoded = base64::engine::general_purpose::STANDARD
                    .encode(format!("{}:{}", username, password));
                req_builder = req_builder.header("Authorization", format!("Basic {}", encoded));
            }
            _ => {}
        }

        match &request.body_ext {
            None => {
                if !sub_body.is_empty() {
                    req_builder = req_builder.body(sub_body.clone());
                }
            }
            Some(BodyExt::FormUrlEncoded { fields }) => {
                let pairs: Vec<(String, String)> = substitute_kvs(fields, env)
                    .into_iter()
                    .filter(|f| f.enabled && !f.key.is_empty())
                    .map(|f| (f.key, f.value))
                    .collect();
                req_builder = req_builder.form(&pairs);
            }
            Some(BodyExt::MultipartForm { fields }) => {
                let mut form = reqwest::multipart::Form::new();
                for f in substitute_kvs(fields, env) {
                    if f.enabled && !f.key.is_empty() {
                        form = form.text(f.key, f.value);
                    }
                }
                req_builder = req_builder.multipart(form);
            }
            Some(BodyExt::GraphQL { variables }) => {
                let vars_value: serde_json::Value =
                    serde_json::from_str(&substitute_vars(variables, env))
                        .unwrap_or(serde_json::json!({}));
                let body_json = serde_json::json!({
                    "query": sub_body,
                    "variables": vars_value,
                });
                req_builder = req_builder.json(&body_json);
            }
        }

        let built = match req_builder.build() {
            Ok(r) => r,
            Err(e) => {
                return ResponseData {
                    body: format!("Error: {}", e),
                    status: "Failed".to_string(),
                    time: "0ms".to_string(),
                    headers: vec![],
                    set_cookies: vec![],
                    response_headers_bytes: 0,
                    response_body_bytes: 0,
                    request_headers_bytes: 0,
                    request_body_bytes: 0,
                    prepare_ms: 0,
                    waiting_ms: 0,
                    download_ms: 0,
                    total_ms: 0,
                };
            }
        };
        let req_line = format!("{} {} HTTP/1.1\r\n", built.method(), built.url().as_str(),);
        let request_headers_bytes = req_line.len()
            + built
                .headers()
                .iter()
                .map(|(k, v)| k.as_str().len() + 2 + v.as_bytes().len() + 2)
                .sum::<usize>()
            + 2;
        let request_body_bytes = built
            .body()
            .and_then(|b| b.as_bytes())
            .map(|b| b.len())
            .unwrap_or(0);

        let t_send = std::time::Instant::now();
        let prepare_ms = t_send
            .saturating_duration_since(t_prepare_start)
            .as_millis() as u64;
        match client.execute(built).await {
            Ok(response) => {
                let t_headers = std::time::Instant::now();
                let waiting_ms = t_headers.saturating_duration_since(t_send).as_millis() as u64;
                let status = format!(
                    "{} {}",
                    response.status().as_u16(),
                    response.status().canonical_reason().unwrap_or("")
                );

                let status_line = format!(
                    "HTTP/1.1 {} {}\r\n",
                    response.status().as_u16(),
                    response.status().canonical_reason().unwrap_or(""),
                );
                let response_headers_bytes = status_line.len()
                    + response
                        .headers()
                        .iter()
                        .map(|(k, v)| k.as_str().len() + 2 + v.as_bytes().len() + 2)
                        .sum::<usize>()
                    + 2;

                let headers: Vec<(String, String)> = response
                    .headers()
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.to_string(),
                            v.to_str().unwrap_or("<non-ascii>").to_string(),
                        )
                    })
                    .collect();

                // Parse every `Set-Cookie` header into a StoredCookie
                // so the caller can merge them into the active env.
                // Host defaults to the request's host if the cookie
                // omits an explicit Domain.
                let set_cookies: Vec<crate::model::StoredCookie> = response
                    .headers()
                    .get_all(reqwest::header::SET_COOKIE)
                    .iter()
                    .filter_map(|v| v.to_str().ok())
                    .filter_map(|s| crate::cookies::parse_set_cookie(s, &url_host))
                    .collect();

                // SSE fork: if Content-Type is text/event-stream, parse
                // incoming chunks as SSE events and emit a Progress
                // update per event. Reader still honors max_body_bytes
                // as a hard safety cap on accumulated event-log size.
                if sse::is_event_stream(&headers) {
                    if let Some(ref prog) = progress {
                        return stream_sse_response(
                            response,
                            headers,
                            set_cookies,
                            status,
                            response_headers_bytes,
                            request_headers_bytes,
                            request_body_bytes,
                            prepare_ms,
                            waiting_ms,
                            t_prepare_start,
                            t_headers,
                            max_body_bytes,
                            prog,
                        )
                        .await;
                    }
                }

                // Stream body with a size cap so a multi-GB payload
                // doesn't OOM the app. We read chunks until we hit
                // `max_body_bytes`, then stop and prepend a banner.
                let (body, truncated) = read_body_capped(response, max_body_bytes).await;
                let body = if truncated {
                    let cap_mb = max_body_bytes as f64 / (1024.0 * 1024.0);
                    format!(
                        "/* Response body truncated at {:.1} MB (see Settings → Max body) */\n{}",
                        cap_mb, body
                    )
                } else {
                    body
                };
                let t_done = std::time::Instant::now();
                let download_ms = t_done.saturating_duration_since(t_headers).as_millis() as u64;
                let total_ms = t_done
                    .saturating_duration_since(t_prepare_start)
                    .as_millis() as u64;
                let time = format!("{} ms", total_ms);
                let response_body_bytes = body.len();
                let mut formatted_body = match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(body),
                    Err(_) => body,
                };
                // Strip trailing whitespace so the JSON gutter doesn't
                // render empty "ghost" line numbers for server-appended
                // newlines. `to_string_pretty` is already clean; this
                // only bites on the raw-body fall-through.
                let trimmed_len = formatted_body.trim_end().len();
                formatted_body.truncate(trimmed_len);

                ResponseData {
                    body: formatted_body,
                    status,
                    time,
                    headers,
                    set_cookies,
                    response_headers_bytes,
                    response_body_bytes,
                    request_headers_bytes,
                    request_body_bytes,
                    prepare_ms,
                    waiting_ms,
                    download_ms,
                    total_ms,
                }
            }
            Err(e) => ResponseData {
                body: format_request_error(&e),
                status: "Failed".to_string(),
                time: "0ms".to_string(),
                headers: vec![],
                set_cookies: vec![],
                response_headers_bytes: 0,
                response_body_bytes: 0,
                request_headers_bytes,
                request_body_bytes,
                prepare_ms,
                waiting_ms: 0,
                download_ms: 0,
                total_ms: prepare_ms,
            },
        }
    }
}

/// Stream an SSE response, emitting a Progress update each time a
/// full event arrives. Returns a Final ResponseData when the stream
/// ends (server closed, cancel via abort, or max_body_bytes hit).
#[allow(clippy::too_many_arguments)]
async fn stream_sse_response(
    mut response: reqwest::Response,
    headers: Vec<(String, String)>,
    set_cookies: Vec<crate::model::StoredCookie>,
    status: String,
    response_headers_bytes: usize,
    request_headers_bytes: usize,
    request_body_bytes: usize,
    prepare_ms: u64,
    waiting_ms: u64,
    t_prepare_start: std::time::Instant,
    t_headers: std::time::Instant,
    max_body_bytes: usize,
    progress: &std::sync::mpsc::Sender<RequestUpdate>,
) -> ResponseData {
    let mut parser = SseParser::new();
    let mut event_log = String::new();
    event_log.push_str("# Streaming SSE — events will appear as they arrive.\n\n");
    let mut event_count: usize = 0;
    let mut body_bytes_seen: usize = 0;
    let mut truncated = false;

    // Send an initial Progress so the UI flips out of "Loading..." as
    // soon as the stream connects.
    let _ = progress.send(RequestUpdate::Progress {
        snapshot: ResponseData {
            body: event_log.clone(),
            status: status.clone(),
            time: format!("{} ms", waiting_ms),
            headers: headers.clone(),
            set_cookies: set_cookies.clone(),
            response_headers_bytes,
            response_body_bytes: 0,
            request_headers_bytes,
            request_body_bytes,
            prepare_ms,
            waiting_ms,
            download_ms: 0,
            total_ms: waiting_ms + prepare_ms,
        },
        new_events: Vec::new(),
    });

    loop {
        match response.chunk().await {
            Ok(Some(bytes)) => {
                body_bytes_seen += bytes.len();
                // Two caps: raw-network-bytes (direct user setting)
                // AND formatted-log-bytes (pretty-printed events can
                // be 3–4× the network size; defend against runaway
                // memory if a server streams millions of tiny events).
                if max_body_bytes > 0 && body_bytes_seen > max_body_bytes {
                    truncated = true;
                    break;
                }
                if max_body_bytes > 0 && event_log.len() > max_body_bytes.saturating_mul(2) {
                    truncated = true;
                    break;
                }
                let events = parser.feed(&bytes);
                if events.is_empty() {
                    continue;
                }
                for ev in &events {
                    event_count += 1;
                    event_log.push_str(&sse::format_event(ev, event_count));
                }
                let elapsed = t_prepare_start.elapsed().as_millis() as u64;
                let _ = progress.send(RequestUpdate::Progress {
                    snapshot: ResponseData {
                        body: event_log.clone(),
                        status: status.clone(),
                        time: format!("{} ms · {} events", elapsed, event_count),
                        headers: headers.clone(),
                        set_cookies: set_cookies.clone(),
                        response_headers_bytes,
                        response_body_bytes: body_bytes_seen,
                        request_headers_bytes,
                        request_body_bytes,
                        prepare_ms,
                        waiting_ms,
                        download_ms: elapsed.saturating_sub(prepare_ms + waiting_ms),
                        total_ms: elapsed,
                    },
                    new_events: events,
                });
            }
            Ok(None) => break, // server closed stream cleanly
            Err(e) => {
                event_log.push_str(&format!("\n── stream error ──\n{}\n", e));
                break;
            }
        }
    }

    if truncated {
        let cap_mb = max_body_bytes as f64 / (1024.0 * 1024.0);
        event_log.push_str(&format!(
            "\n── truncated ──\nEvent log exceeded {:.1} MB (see Settings → Max body).\n",
            cap_mb
        ));
    }

    let t_done = std::time::Instant::now();
    let download_ms = t_done.saturating_duration_since(t_headers).as_millis() as u64;
    let total_ms = t_done
        .saturating_duration_since(t_prepare_start)
        .as_millis() as u64;
    ResponseData {
        body: event_log,
        status,
        time: format!("{} ms · {} events", total_ms, event_count),
        headers,
        set_cookies,
        response_headers_bytes,
        response_body_bytes: body_bytes_seen,
        request_headers_bytes,
        request_body_bytes,
        prepare_ms,
        waiting_ms,
        download_ms,
        total_ms,
    }
}

/// Read a response body into a String, stopping as soon as `max_bytes`
/// is reached. Returns `(body, truncated)`. A cap of `0` disables the
/// limit and reads the whole response (equivalent to `.text()`).
///
/// Uses `reqwest::Response::chunk()` so we pull incrementally from the
/// network and abort early on oversize payloads — no 5 GB allocation
/// just because a server returned a 5 GB body.
async fn read_body_capped(mut response: reqwest::Response, max_bytes: usize) -> (String, bool) {
    if max_bytes == 0 {
        let body = response
            .text()
            .await
            .unwrap_or_else(|e| format!("Error reading body: {}", e));
        return (body, false);
    }
    let mut buf: Vec<u8> = Vec::new();
    let mut truncated = false;
    loop {
        match response.chunk().await {
            Ok(Some(bytes)) => {
                if buf.len() + bytes.len() > max_bytes {
                    let remaining = max_bytes.saturating_sub(buf.len());
                    buf.extend_from_slice(&bytes[..remaining]);
                    truncated = true;
                    break;
                }
                buf.extend_from_slice(&bytes);
            }
            Ok(None) => break,
            Err(e) => {
                let tail = format!("\n/* Error reading chunk: {} */", e);
                buf.extend_from_slice(tail.as_bytes());
                break;
            }
        }
    }
    // Lossy — response may stop mid-UTF-8; safer than failing.
    (String::from_utf8_lossy(&buf).into_owned(), truncated)
}
