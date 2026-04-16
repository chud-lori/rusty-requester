//! OAuth 2.0 Authorization Code + PKCE flow.
//!
//! Scope: covers the flow used by most public / SPA / native
//! clients — no client secret required, PKCE protects the code
//! exchange. Does NOT implement Client Credentials, Resource Owner
//! Password, Device Code, or JWT-bearer flows; those can follow
//! in later 1.x releases without schema breakage.
//!
//! End-to-end shape of a flow:
//!
//!   1. UI calls `begin_flow(config)` which returns a `FlowHandle`
//!      holding the PKCE verifier, state parameter, and a local
//!      `TcpListener` bound to `127.0.0.1:<random>`.
//!   2. `FlowHandle::authorize_url()` produces the provider URL;
//!      the caller `open_in_browser`s it.
//!   3. `FlowHandle::wait_for_redirect(timeout)` blocks until the
//!      provider redirects the user's browser to our listener.
//!      Returns the `code` parameter (or an error if state mismatch,
//!      provider error, timeout).
//!   4. `exchange_code(config, code, verifier)` POSTs to the
//!      token endpoint, returning `TokenResponse`.
//!
//! Thread safety: the listener blocks the calling thread, so
//! callers should run the flow on a background thread (`thread::spawn`
//! is fine — this isn't CPU-bound) and surface the result through
//! an `mpsc::channel`.

use base64::Engine;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use crate::model::OAuth2Config;

#[derive(Debug)]
pub enum OAuthError {
    Bind(String),
    Timeout,
    StateMismatch,
    ProviderError(String),
    Network(String),
    Parse(String),
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Bind(e) => write!(f, "could not bind local callback listener: {}", e),
            Self::Timeout => write!(f, "timed out waiting for the provider to redirect"),
            Self::StateMismatch => write!(
                f,
                "state parameter mismatch — the redirect was not from our flow"
            ),
            Self::ProviderError(m) => write!(f, "provider rejected the request: {}", m),
            Self::Network(m) => write!(f, "network error: {}", m),
            Self::Parse(m) => write!(f, "could not parse provider response: {}", m),
        }
    }
}

impl std::error::Error for OAuthError {}

#[derive(Debug, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in_secs: Option<i64>,
}

/// PKCE code verifier (43–128 unreserved chars per RFC 7636). We
/// use 32 bytes (256 bits) of entropy → 43 base64url chars.
fn generate_pkce_verifier() -> String {
    // 32 bytes of entropy by concatenating two UUID v4s. `uuid` is
    // already a dependency and its v4 uses OS-provided CSPRNG, so
    // this is cryptographically sound without pulling in `rand` /
    // `getrandom` as direct deps.
    let a = uuid::Uuid::new_v4();
    let b = uuid::Uuid::new_v4();
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(a.as_bytes());
    bytes[16..].copy_from_slice(b.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Derive the S256 code challenge from a verifier.
fn code_challenge_s256(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

/// Opaque state token — prevents cross-site replay of the redirect.
fn generate_state() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub struct FlowHandle {
    listener: TcpListener,
    redirect_uri: String,
    state: String,
    pub verifier: String,
    pub challenge: String,
}

/// Begin a PKCE flow. Binds a local TCP listener on `127.0.0.1`
/// (random port) and returns the handle. The caller should
/// immediately call `authorize_url()` and open it in the user's
/// browser.
pub fn begin_flow(config: &OAuth2Config) -> Result<FlowHandle, OAuthError> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| OAuthError::Bind(e.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|e| OAuthError::Bind(e.to_string()))?
        .port();
    // We override the user's registered redirect_uri port with the
    // port we just bound — PKCE-capable providers generally allow
    // any 127.0.0.1 port. Providers that require exact match need
    // the user to register our exact URI; we provide the default
    // `http://127.0.0.1/callback` as a placeholder that the user
    // can refine.
    let redirect_uri = {
        let base = config.redirect_uri.trim_end_matches('/');
        if base.contains("127.0.0.1") || base.contains("localhost") {
            // Inject the port while preserving any path the user set.
            rewrite_loopback_port(base, port)
        } else {
            base.to_string()
        }
    };
    let verifier = generate_pkce_verifier();
    let challenge = code_challenge_s256(&verifier);
    let state = generate_state();
    Ok(FlowHandle {
        listener,
        redirect_uri,
        state,
        verifier,
        challenge,
    })
}

/// Replace (or inject) the port on a `http://host[:port][/path]` URL.
/// Used to pin the callback URL to the port we actually bound.
fn rewrite_loopback_port(url: &str, port: u16) -> String {
    let (scheme, rest) = match url.split_once("://") {
        Some((s, r)) => (s, r),
        None => ("http", url),
    };
    let (host_and_port, path) = match rest.split_once('/') {
        Some((hp, p)) => (hp, format!("/{}", p)),
        None => (rest, String::new()),
    };
    let host = host_and_port.split(':').next().unwrap_or(host_and_port);
    format!("{}://{}:{}{}", scheme, host, port, path)
}

impl FlowHandle {
    pub fn authorize_url(&self, config: &OAuth2Config) -> String {
        let mut params: Vec<(&str, String)> = vec![
            ("response_type", "code".into()),
            ("client_id", config.client_id.clone()),
            ("redirect_uri", self.redirect_uri.clone()),
            ("state", self.state.clone()),
            ("code_challenge", self.challenge.clone()),
            ("code_challenge_method", "S256".into()),
        ];
        if !config.scope.trim().is_empty() {
            params.push(("scope", config.scope.clone()));
        }
        let query: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let sep = if config.auth_url.contains('?') {
            '&'
        } else {
            '?'
        };
        format!("{}{}{}", config.auth_url.trim_end_matches('&'), sep, query)
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Block until the user's browser hits our callback listener, or
    /// the timeout elapses. Returns the authorization `code` value.
    pub fn wait_for_redirect(&self, timeout: Duration) -> Result<String, OAuthError> {
        self.listener
            .set_nonblocking(false)
            .map_err(|e| OAuthError::Bind(e.to_string()))?;
        // Per-accept read timeout so an unresponsive browser doesn't
        // wedge the listener forever. Loop until we see a GET on
        // the callback path or exceed `timeout`.
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(OAuthError::Timeout);
            }
            self.listener
                .set_nonblocking(true)
                .map_err(|e| OAuthError::Bind(e.to_string()))?;
            match self.listener.accept() {
                Ok((mut stream, _addr)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .map_err(|e| OAuthError::Network(e.to_string()))?;
                    let mut buf = [0u8; 4096];
                    let n = stream
                        .read(&mut buf)
                        .map_err(|e| OAuthError::Network(e.to_string()))?;
                    let req = String::from_utf8_lossy(&buf[..n]);
                    // Reply with a browser-friendly page telling the
                    // user they can close the window. Must write
                    // before we drop `stream`, otherwise the browser
                    // shows "connection reset".
                    let body = b"<!doctype html><html><body style='font-family:system-ui;padding:40px;text-align:center'><h2>\xE2\x9C\x93 Authentication complete</h2><p>You can close this window and return to Rusty Requester.</p></body></html>";
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.write_all(body);
                    let _ = stream.flush();
                    return self.parse_callback_query(&req);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(200));
                    continue;
                }
                Err(e) => return Err(OAuthError::Network(e.to_string())),
            }
        }
    }

    fn parse_callback_query(&self, request: &str) -> Result<String, OAuthError> {
        // Request line: `GET /callback?code=...&state=... HTTP/1.1`
        let first_line = request.lines().next().unwrap_or("");
        let path = first_line.split_whitespace().nth(1).unwrap_or("");
        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
        let mut code: Option<String> = None;
        let mut state: Option<String> = None;
        let mut error: Option<String> = None;
        let mut error_desc: Option<String> = None;
        for pair in query.split('&') {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let v = url_decode(v);
            match k {
                "code" => code = Some(v),
                "state" => state = Some(v),
                "error" => error = Some(v),
                "error_description" => error_desc = Some(v),
                _ => {}
            }
        }
        if let Some(e) = error {
            let msg = error_desc
                .map(|d| format!("{}: {}", e, d))
                .unwrap_or_else(|| e);
            return Err(OAuthError::ProviderError(msg));
        }
        if state.as_deref() != Some(self.state.as_str()) {
            return Err(OAuthError::StateMismatch);
        }
        code.ok_or_else(|| OAuthError::Parse("missing `code` parameter".into()))
    }
}

/// Exchange an authorization code for an access token.
pub async fn exchange_code(
    client: &reqwest::Client,
    config: &OAuth2Config,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, OAuthError> {
    let mut form: Vec<(&str, &str)> = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", config.client_id.as_str()),
        ("code_verifier", verifier),
    ];
    if !config.client_secret.is_empty() {
        form.push(("client_secret", config.client_secret.as_str()));
    }
    let resp = client
        .post(&config.token_url)
        .form(&form)
        .send()
        .await
        .map_err(|e| OAuthError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(OAuthError::ProviderError(format!(
            "token endpoint returned {}: {}",
            status, body
        )));
    }
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| OAuthError::Parse(e.to_string()))?;
    parse_token_response(&json)
}

fn parse_token_response(json: &serde_json::Value) -> Result<TokenResponse, OAuthError> {
    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| OAuthError::Parse("missing `access_token` in response".into()))?
        .to_string();
    let refresh_token = json
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let expires_in_secs = json.get("expires_in").and_then(|v| v.as_i64());
    Ok(TokenResponse {
        access_token,
        refresh_token,
        expires_in_secs,
    })
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_verifier_length_in_range() {
        let v = generate_pkce_verifier();
        // 32 bytes → 43 base64url chars (no padding).
        assert_eq!(v.len(), 43);
        // Only unreserved chars per RFC 7636.
        assert!(v
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn s256_challenge_is_deterministic() {
        // Canonical RFC 7636 Appendix B vector:
        //   verifier  = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        //   challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        let v = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let c = code_challenge_s256(v);
        assert_eq!(c, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn authorize_url_encodes_params() {
        let cfg = OAuth2Config {
            auth_url: "https://example.com/authorize".into(),
            token_url: "https://example.com/token".into(),
            client_id: "abc 123".into(),
            client_secret: String::new(),
            scope: "read write".into(),
            redirect_uri: "http://127.0.0.1/callback".into(),
        };
        let fh = FlowHandle {
            listener: TcpListener::bind("127.0.0.1:0").unwrap(),
            redirect_uri: "http://127.0.0.1:54321/callback".into(),
            state: "s".into(),
            verifier: "v".into(),
            challenge: "ch".into(),
        };
        let u = fh.authorize_url(&cfg);
        assert!(u.starts_with("https://example.com/authorize?"));
        assert!(u.contains("response_type=code"));
        assert!(u.contains("client_id=abc%20123"));
        assert!(u.contains("scope=read%20write"));
        assert!(u.contains("code_challenge=ch"));
        assert!(u.contains("code_challenge_method=S256"));
        assert!(u.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A54321%2Fcallback"));
    }

    #[test]
    fn rewrite_loopback_port_preserves_path() {
        let out = rewrite_loopback_port("http://127.0.0.1/callback", 54321);
        assert_eq!(out, "http://127.0.0.1:54321/callback");
        let out2 = rewrite_loopback_port("http://localhost:8080/cb", 54321);
        assert_eq!(out2, "http://localhost:54321/cb");
        let out3 = rewrite_loopback_port("http://127.0.0.1", 54321);
        assert_eq!(out3, "http://127.0.0.1:54321");
    }

    #[test]
    fn parses_token_response() {
        let json = serde_json::json!({
            "access_token": "at",
            "refresh_token": "rt",
            "expires_in": 3600,
            "token_type": "Bearer"
        });
        let t = parse_token_response(&json).unwrap();
        assert_eq!(t.access_token, "at");
        assert_eq!(t.refresh_token, "rt");
        assert_eq!(t.expires_in_secs, Some(3600));
    }

    #[test]
    fn parses_token_response_missing_refresh() {
        let json = serde_json::json!({
            "access_token": "at",
            "expires_in": 3600
        });
        let t = parse_token_response(&json).unwrap();
        assert_eq!(t.access_token, "at");
        assert_eq!(t.refresh_token, "");
    }
}
