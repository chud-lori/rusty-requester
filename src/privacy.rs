const MASK_SHORT_MAX: usize = 8;
const MASK_LONG_PREFIX: usize = 6;
const MASK_LONG_SUFFIX: usize = 4;
pub const REDACTED_VALUE: &str = "<redacted>";

/// Returns true for common header, cookie, query, and form keys that
/// usually carry credentials or session material.
pub fn is_sensitive_key(key: &str) -> bool {
    let normalized = normalize_key(key);
    if normalized.is_empty() {
        return false;
    }

    matches!(
        normalized.as_str(),
        "authorization"
            | "proxyauthorization"
            | "cookie"
            | "setcookie"
            | "apikey"
            | "xapikey"
            | "accesskey"
            | "secretkey"
            | "clientsecret"
            | "password"
            | "passwd"
            | "pwd"
            | "sessionid"
            | "csrftoken"
            | "xcsrftoken"
            | "xsrf"
            | "xsrftoken"
    ) || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("password")
        || normalized.contains("session")
}

/// Redacts any URL query string and fragment content for display. The
/// path remains visible, but query and fragment payloads become `...`.
pub fn redact_url_query_and_fragment(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let (without_fragment, fragment) = match trimmed.split_once('#') {
        Some((base, _)) => (base, Some("#...")),
        None => (trimmed, None),
    };
    let mut out = match without_fragment.split_once('?') {
        Some((base, query)) if !query.is_empty() => format!("{}?...", base),
        Some((base, _)) => base.to_string(),
        None => without_fragment.to_string(),
    };
    if let Some(fragment) = fragment {
        out.push_str(fragment);
    }
    out
}

/// Masks a secret-ish value for display while keeping enough shape to
/// distinguish long values. Empty stays empty.
pub fn mask_secret_value(value: &str) -> String {
    let len = value.chars().count();
    if len == 0 {
        return String::new();
    }
    if len <= MASK_SHORT_MAX {
        return "*".repeat(len);
    }

    let prefix: String = value.chars().take(MASK_LONG_PREFIX.min(len / 2)).collect();
    let suffix_len = MASK_LONG_SUFFIX.min(len.saturating_sub(prefix.chars().count()));
    let suffix_rev: String = value.chars().rev().take(suffix_len).collect();
    let suffix: String = suffix_rev.chars().rev().collect();
    format!("{}...{}", prefix, suffix)
}

/// Redact a value when its key commonly carries credentials.
pub fn redact_sensitive_value(key: &str, value: &str) -> String {
    if is_sensitive_key(key) {
        REDACTED_VALUE.to_string()
    } else {
        value.to_string()
    }
}

/// Redact request header values while preserving enough protocol shape
/// to keep generated snippets useful.
pub fn redact_header_value(key: &str, value: &str) -> String {
    if !is_sensitive_key(key) {
        return value.to_string();
    }

    let normalized = normalize_key(key);
    if normalized == "authorization" || normalized == "proxyauthorization" {
        return redact_authorization_value(value);
    }
    if normalized == "cookie" || normalized == "setcookie" {
        return redact_cookie_header_value(value);
    }

    REDACTED_VALUE.to_string()
}

/// Redact URL query values whose keys look sensitive and hide fragments.
pub fn redact_url_sensitive_query_values(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let (without_fragment, fragment) = match trimmed.split_once('#') {
        Some((base, frag)) if !frag.is_empty() => (base, Some("#...")),
        Some((base, _)) => (base, Some("#")),
        None => (trimmed, None),
    };

    let mut out = match without_fragment.split_once('?') {
        Some((base, query)) if !query.is_empty() => {
            let redacted_query = query
                .split('&')
                .map(redact_query_part)
                .collect::<Vec<_>>()
                .join("&");
            format!("{}?{}", base, redacted_query)
        }
        Some((base, _)) => format!("{}?", base),
        None => without_fragment.to_string(),
    };
    if let Some(fragment) = fragment {
        out.push_str(fragment);
    }
    out
}

/// Redact sensitive JSON or form-like body fields. Bodies that do not
/// expose key/value structure are returned unchanged.
pub fn redact_body_text(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(body) {
        redact_json_value(&mut value, None);
        return serde_json::to_string_pretty(&value).unwrap_or_else(|_| body.to_string());
    }

    if looks_like_form_body(body) {
        return body
            .split('&')
            .map(redact_query_part)
            .collect::<Vec<_>>()
            .join("&");
    }

    body.to_string()
}

/// Escape text for safe insertion into HTML text/attribute contexts.
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn redact_authorization_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    match trimmed.split_once(char::is_whitespace) {
        Some((scheme, _)) if !scheme.is_empty() => format!("{} {}", scheme, REDACTED_VALUE),
        _ => REDACTED_VALUE.to_string(),
    }
}

fn redact_cookie_header_value(value: &str) -> String {
    value
        .split(';')
        .map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return String::new();
            }
            match trimmed.split_once('=') {
                Some((name, _)) if !name.trim().is_empty() => {
                    format!("{}={}", name.trim(), REDACTED_VALUE)
                }
                _ => REDACTED_VALUE.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn redact_query_part(part: &str) -> String {
    match part.split_once('=') {
        Some((key, value)) => {
            let redacted = redact_sensitive_value(key, value);
            format!("{}={}", key, redacted)
        }
        None if is_sensitive_key(part) => format!("{}={}", part, REDACTED_VALUE),
        None => part.to_string(),
    }
}

fn looks_like_form_body(body: &str) -> bool {
    body.split('&').any(|part| {
        part.split_once('=')
            .map(|(key, _)| is_sensitive_key(key))
            .unwrap_or(false)
    })
}

fn redact_json_value(value: &mut serde_json::Value, parent_key: Option<&str>) {
    if parent_key.is_some_and(is_sensitive_key) {
        *value = serde_json::Value::String(REDACTED_VALUE.to_string());
        return;
    }

    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                redact_json_value(child, Some(key));
            }
        }
        serde_json::Value::Array(items) => {
            for child in items {
                redact_json_value(child, None);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sensitive_keys_across_common_spellings() {
        for key in [
            "Authorization",
            "X-API-Key",
            "access_token",
            "client-secret",
            "password",
            "session_id",
            "csrf-token",
            "Set-Cookie",
        ] {
            assert!(is_sensitive_key(key), "{key}");
        }
    }

    #[test]
    fn leaves_plain_keys_non_sensitive() {
        for key in ["page", "content-type", "user_name", "request_id"] {
            assert!(!is_sensitive_key(key), "{key}");
        }
    }

    #[test]
    fn url_redaction_hides_query_values() {
        let url = "https://api.example.com/users?api_key=secret&email=a@example.com";
        assert_eq!(
            redact_url_query_and_fragment(url),
            "https://api.example.com/users?..."
        );
    }

    #[test]
    fn url_sensitive_redaction_preserves_non_secret_query_values() {
        let url = "https://api.example.com/users?page=2&api_key=secret#access_token=secret";
        assert_eq!(
            redact_url_sensitive_query_values(url),
            "https://api.example.com/users?page=2&api_key=<redacted>#..."
        );
    }

    #[test]
    fn header_redaction_preserves_auth_and_cookie_shape() {
        assert_eq!(
            redact_header_value("Authorization", "Bearer abc123"),
            "Bearer <redacted>"
        );
        assert_eq!(
            redact_header_value("Cookie", "sid=abc; theme=dark"),
            "sid=<redacted>; theme=<redacted>"
        );
        assert_eq!(redact_header_value("X-Trace", "abc123"), "abc123");
    }

    #[test]
    fn body_redaction_handles_json_and_form_keys() {
        assert_eq!(
            redact_body_text(r#"{"user":"ada","password":"secret","nested":{"token":"abc"}}"#),
            "{\n  \"nested\": {\n    \"token\": \"<redacted>\"\n  },\n  \"password\": \"<redacted>\",\n  \"user\": \"ada\"\n}"
        );
        assert_eq!(
            redact_body_text("username=ada&access_token=abc"),
            "username=ada&access_token=<redacted>"
        );
    }

    #[test]
    fn url_redaction_hides_fragment() {
        let url = "https://api.example.com/users#access_token=secret";
        assert_eq!(
            redact_url_query_and_fragment(url),
            "https://api.example.com/users#..."
        );
    }

    #[test]
    fn url_redaction_hides_query_and_fragment_together() {
        let url = "https://api.example.com/users?token=secret#refresh=secret";
        assert_eq!(
            redact_url_query_and_fragment(url),
            "https://api.example.com/users?...#..."
        );
    }

    #[test]
    fn mask_secret_value_masks_short_values_completely() {
        assert_eq!(mask_secret_value("abc123"), "******");
    }

    #[test]
    fn mask_secret_value_keeps_long_value_shape() {
        assert_eq!(
            mask_secret_value("abcdefghijklmnopqrstuvwxyz"),
            "abcdef...wxyz"
        );
    }

    #[test]
    fn escape_html_escapes_text_and_attribute_breakouts() {
        assert_eq!(
            escape_html(r#"<script x="1">Tom & 'Jerry'</script>"#),
            "&lt;script x=&quot;1&quot;&gt;Tom &amp; &#39;Jerry&#39;&lt;/script&gt;"
        );
    }
}
