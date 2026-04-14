//! Network layer — HTTP request execution, URL normalization, and
//! error formatting. Kept free of UI concerns so it can be unit-tested
//! independently of egui.

use crate::io::curl;
use crate::model::{Auth, BodyExt, Environment, HttpMethod, Request, ResponseData};
use crate::widgets::{substitute_kvs, substitute_vars};
use base64::Engine;

/// Ensure the URL has a scheme so reqwest can parse it. Defaults to
/// `http://` — matches curl's historical default and Postman's
/// behavior for schemeless URLs. Users hitting HTTPS-only services
/// can type `https://` explicitly.
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

/// Fire a single HTTP request (synchronously; spawns its own tokio
/// runtime per call) and return a fully-populated `ResponseData` —
/// body, headers, status, size breakdown, and phase timings. Designed
/// to run off the UI thread via `poll_promise::Promise::spawn_thread`.
pub fn execute_request(request: &Request, env: Option<&Environment>) -> ResponseData {
    let rt = tokio::runtime::Runtime::new().unwrap();
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
    };

    rt.block_on(async {
        let client = reqwest::Client::new();
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

        let mut cookie_parts: Vec<String> = Vec::new();
        for h in &sub_headers {
            if !h.enabled || h.key.trim().is_empty() {
                continue;
            }
            if h.key.eq_ignore_ascii_case("cookie") {
                cookie_parts.push(h.value.clone());
                continue;
            }
            req_builder = req_builder.header(&h.key, &h.value);
        }
        for c in &sub_cookies {
            if c.enabled && !c.key.is_empty() {
                cookie_parts.push(format!("{}={}", c.key, c.value));
            }
        }
        if !cookie_parts.is_empty() {
            req_builder = req_builder.header("Cookie", cookie_parts.join("; "));
        }

        match &sub_auth {
            Auth::Bearer { token } if !token.is_empty() => {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
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
        let req_line = format!(
            "{} {} HTTP/1.1\r\n",
            built.method(),
            built.url().as_str(),
        );
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
        let prepare_ms = t_send.saturating_duration_since(t_prepare_start).as_millis() as u64;
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

                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("Error reading body: {}", e));
                let t_done = std::time::Instant::now();
                let download_ms = t_done.saturating_duration_since(t_headers).as_millis() as u64;
                let total_ms = t_done.saturating_duration_since(t_prepare_start).as_millis() as u64;
                let time = format!("{} ms", total_ms);
                let response_body_bytes = body.len();
                let formatted_body = match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(body),
                    Err(_) => body,
                };

                ResponseData {
                    body: formatted_body,
                    status,
                    time,
                    headers,
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
    })
}
