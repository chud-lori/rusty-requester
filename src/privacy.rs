const MASK_SHORT_MAX: usize = 8;
const MASK_LONG_PREFIX: usize = 6;
const MASK_LONG_SUFFIX: usize = 4;

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
