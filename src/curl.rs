use crate::{Auth, HttpMethod, Request};
use uuid::Uuid;

pub fn to_curl(req: &Request) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push("curl".to_string());
    parts.push(format!("-X {}", req.method));

    let full_url = build_full_url(&req.url, &req.query_params);
    parts.push(format!("'{}'", esc(&full_url)));

    for (k, v) in &req.headers {
        if k.trim().is_empty() {
            continue;
        }
        parts.push(format!("-H '{}: {}'", esc(k), esc(v)));
    }

    match &req.auth {
        Auth::Bearer { token } if !token.is_empty() => {
            parts.push(format!("-H 'Authorization: Bearer {}'", esc(token)));
        }
        Auth::Basic { username, password } if !username.is_empty() => {
            parts.push(format!("-u '{}:{}'", esc(username), esc(password)));
        }
        _ => {}
    }

    if !req.body.is_empty() {
        parts.push(format!("--data-raw '{}'", esc(&req.body)));
    }

    parts.join(" \\\n  ")
}

pub fn build_full_url(base: &str, params: &[(String, String)]) -> String {
    let enabled: Vec<&(String, String)> = params.iter().filter(|(k, _)| !k.is_empty()).collect();
    if enabled.is_empty() {
        return base.to_string();
    }
    let query = enabled
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let sep = if base.contains('?') { '&' } else { '?' };
    format!("{}{}{}", base, sep, query)
}

fn esc(s: &str) -> String {
    s.replace('\'', "'\\''")
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

pub fn parse_curl(input: &str) -> Result<Request, String> {
    let tokens = tokenize(input).map_err(|e| format!("Tokenize error: {}", e))?;
    if tokens.is_empty() {
        return Err("Empty input".to_string());
    }

    let mut method: Option<HttpMethod> = None;
    let mut url: Option<String> = None;
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut body = String::new();
    let mut auth = Auth::None;
    let mut data_given = false;

    let start = if tokens[0].eq_ignore_ascii_case("curl") { 1 } else { 0 };
    let mut i = start;
    while i < tokens.len() {
        let tok = &tokens[i];
        match tok.as_str() {
            "-X" | "--request" => {
                i += 1;
                if i < tokens.len() {
                    method = Some(parse_method(&tokens[i]));
                }
            }
            "-H" | "--header" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((k, v)) = split_header(&tokens[i]) {
                        headers.push((k, v));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
                i += 1;
                if i < tokens.len() {
                    let val = &tokens[i];
                    if val.starts_with('@') {
                        body = format!("// file ref skipped: {}", val);
                    } else {
                        body = val.clone();
                    }
                    data_given = true;
                }
            }
            "--data-urlencode" => {
                i += 1;
                if i < tokens.len() {
                    body = tokens[i].clone();
                    data_given = true;
                }
            }
            "-u" | "--user" => {
                i += 1;
                if i < tokens.len() {
                    let val = &tokens[i];
                    let (u, p) = match val.split_once(':') {
                        Some((u, p)) => (u.to_string(), p.to_string()),
                        None => (val.clone(), String::new()),
                    };
                    auth = Auth::Basic { username: u, password: p };
                }
            }
            "--url" => {
                i += 1;
                if i < tokens.len() {
                    url = Some(tokens[i].clone());
                }
            }
            "-A" | "--user-agent" => {
                i += 1;
                if i < tokens.len() {
                    headers.push(("User-Agent".to_string(), tokens[i].clone()));
                }
            }
            "-b" | "--cookie" => {
                i += 1;
                if i < tokens.len() {
                    headers.push(("Cookie".to_string(), tokens[i].clone()));
                }
            }
            "-e" | "--referer" => {
                i += 1;
                if i < tokens.len() {
                    headers.push(("Referer".to_string(), tokens[i].clone()));
                }
            }
            "-I" | "--head" => {
                method = Some(HttpMethod::HEAD);
            }
            "-G" | "--get" => {
                method = Some(HttpMethod::GET);
            }
            // Flags without args — ignore
            "-L" | "--location" | "-k" | "--insecure" | "--compressed" | "-s" | "--silent"
            | "-v" | "--verbose" | "-i" | "--include" | "-f" | "--fail" => {}
            // Flags with one ignored arg
            "-o" | "--output" | "-m" | "--max-time" | "--connect-timeout" | "--resolve"
            | "-w" | "--write-out" | "-x" | "--proxy" => {
                i += 1;
            }
            s if s.starts_with("--") || (s.starts_with('-') && s.len() > 1) => {
                // Unknown flag — skip; if next token doesn't look like a flag/URL, skip it too
            }
            _ => {
                if url.is_none() {
                    url = Some(tok.clone());
                }
            }
        }
        i += 1;
    }

    let full_url = url.ok_or_else(|| "No URL found".to_string())?;
    let (base_url, query_params) = split_url(&full_url);

    let method = method.unwrap_or(if data_given { HttpMethod::POST } else { HttpMethod::GET });

    // Detect Bearer in Authorization header
    let mut filtered_headers: Vec<(String, String)> = Vec::new();
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("Authorization") {
            let trimmed = v.trim();
            if let Some(rest) = trimmed.strip_prefix("Bearer ").or_else(|| trimmed.strip_prefix("bearer ")) {
                if matches!(auth, Auth::None) {
                    auth = Auth::Bearer { token: rest.to_string() };
                    continue;
                }
            }
        }
        filtered_headers.push((k, v));
    }

    Ok(Request {
        id: Uuid::new_v4().to_string(),
        name: "Imported from cURL".to_string(),
        method,
        url: base_url,
        query_params,
        headers: filtered_headers,
        body,
        auth,
    })
}

fn parse_method(s: &str) -> HttpMethod {
    match s.to_ascii_uppercase().as_str() {
        "GET" => HttpMethod::GET,
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "DELETE" => HttpMethod::DELETE,
        "PATCH" => HttpMethod::PATCH,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    }
}

fn split_header(s: &str) -> Option<(String, String)> {
    let (k, v) = s.split_once(':')?;
    Some((k.trim().to_string(), v.trim().to_string()))
}

fn split_url(full: &str) -> (String, Vec<(String, String)>) {
    match full.split_once('?') {
        None => (full.to_string(), Vec::new()),
        Some((base, query)) => {
            let params = query
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| match p.split_once('=') {
                    Some((k, v)) => (url_decode(k), url_decode(v)),
                    None => (url_decode(p), String::new()),
                })
                .collect();
            (base.to_string(), params)
        }
    }
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

fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut has_content = false;

    while let Some(c) = chars.next() {
        if in_single {
            if c == '\'' {
                in_single = false;
            } else {
                cur.push(c);
            }
            continue;
        }
        if in_double {
            match c {
                '"' => in_double = false,
                '\\' => {
                    if let Some(&next) = chars.peek() {
                        match next {
                            '"' | '\\' | '$' | '`' | '\n' => {
                                chars.next();
                                if next != '\n' {
                                    cur.push(next);
                                }
                            }
                            _ => cur.push('\\'),
                        }
                    }
                }
                _ => cur.push(c),
            }
            continue;
        }
        match c {
            '\'' => {
                in_single = true;
                has_content = true;
            }
            '"' => {
                in_double = true;
                has_content = true;
            }
            '\\' => {
                if let Some(&next) = chars.peek() {
                    if next == '\n' || next == '\r' {
                        chars.next();
                        if next == '\r' {
                            if let Some(&'\n') = chars.peek() {
                                chars.next();
                            }
                        }
                        continue;
                    }
                    chars.next();
                    cur.push(next);
                    has_content = true;
                }
            }
            c if c.is_whitespace() => {
                if has_content {
                    tokens.push(std::mem::take(&mut cur));
                    has_content = false;
                }
            }
            _ => {
                cur.push(c);
                has_content = true;
            }
        }
    }
    if in_single || in_double {
        return Err("Unclosed quote".to_string());
    }
    if has_content {
        tokens.push(cur);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple() {
        let toks = tokenize("curl -X POST 'https://x.com' -H 'A: B'").unwrap();
        assert_eq!(toks, vec!["curl", "-X", "POST", "https://x.com", "-H", "A: B"]);
    }

    #[test]
    fn parse_basic_get() {
        let r = parse_curl("curl https://example.com/path?a=1&b=2").unwrap();
        assert_eq!(r.method, HttpMethod::GET);
        assert_eq!(r.url, "https://example.com/path");
        assert_eq!(r.query_params.len(), 2);
    }

    #[test]
    fn parse_post_with_body_and_bearer() {
        let r = parse_curl(
            "curl -X POST 'https://api.example.com/v1/thing' \\\n  -H 'Content-Type: application/json' \\\n  -H 'Authorization: Bearer abc123' \\\n  -d '{\"k\":\"v\"}'",
        )
        .unwrap();
        assert_eq!(r.method, HttpMethod::POST);
        assert_eq!(r.body, "{\"k\":\"v\"}");
        assert!(matches!(r.auth, Auth::Bearer { .. }));
        assert_eq!(r.headers.len(), 1);
    }

    #[test]
    fn parse_data_implies_post() {
        let r = parse_curl("curl https://x.com -d 'hello'").unwrap();
        assert_eq!(r.method, HttpMethod::POST);
        assert_eq!(r.body, "hello");
    }

    #[test]
    fn to_curl_round_trip_shape() {
        let r = Request {
            id: "x".into(),
            name: "n".into(),
            method: HttpMethod::POST,
            url: "https://a.com".into(),
            query_params: vec![("q".into(), "1".into())],
            headers: vec![("X-Foo".into(), "bar".into())],
            body: "{}".into(),
            auth: Auth::None,
        };
        let s = to_curl(&r);
        assert!(s.contains("curl"));
        assert!(s.contains("-X POST"));
        assert!(s.contains("https://a.com?q=1"));
        assert!(s.contains("-H 'X-Foo: bar'"));
        assert!(s.contains("--data-raw '{}'"));
    }
}
