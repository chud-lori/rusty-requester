//! Minimal RFC 6265-ish cookie jar. Persists `Set-Cookie` responses
//! into the active environment and merges matching cookies back into
//! the next request's `Cookie` header.
//!
//! Intentionally NOT a full RFC 6265 implementation — we skip
//! `SameSite`, `Priority`, `Partitioned`, public-suffix matching, etc.
//! All we need is: "server set a cookie for api.foo.com, send it the
//! next time we hit api.foo.com/anything". Good enough for an API
//! client.

use crate::model::StoredCookie;

/// Parse a single `Set-Cookie` header value into a `StoredCookie`.
/// Skips malformed inputs (returns `None`). The `request_host` is the
/// host the response came from — used as the cookie's default
/// `domain` if none was specified.
pub fn parse_set_cookie(header_value: &str, request_host: &str) -> Option<StoredCookie> {
    let mut parts = header_value.splitn(2, ';');
    let name_value = parts.next()?;
    let (name, value) = name_value.split_once('=')?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return None;
    }
    let value = value.trim().to_string();

    let mut cookie = StoredCookie {
        name,
        value,
        domain: request_host.to_ascii_lowercase(),
        path: "/".to_string(),
        expires: None,
        secure: false,
        http_only: false,
    };

    for attr in parts.next().unwrap_or_default().split(';') {
        let attr = attr.trim();
        if attr.is_empty() {
            continue;
        }
        let (k, v) = attr.split_once('=').unwrap_or((attr, ""));
        let k = k.trim().to_ascii_lowercase();
        let v = v.trim();
        match k.as_str() {
            "domain" => {
                // Spec says leading dot is ignored; treat everything
                // as host-suffix match.
                let d = v.trim_start_matches('.').to_ascii_lowercase();
                if !d.is_empty() {
                    cookie.domain = d;
                }
            }
            "path" => {
                if !v.is_empty() {
                    cookie.path = v.to_string();
                }
            }
            "expires" => {
                if let Some(ts) = parse_http_date(v) {
                    cookie.expires = Some(ts);
                }
            }
            "max-age" => {
                // Max-Age wins over Expires per RFC. `i64` seconds
                // from now; negative = delete.
                if let Ok(s) = v.parse::<i64>() {
                    let now = now_epoch();
                    cookie.expires = Some(now.saturating_add(s));
                }
            }
            "secure" => cookie.secure = true,
            "httponly" => cookie.http_only = true,
            _ => {} // Ignore SameSite, Priority, etc.
        }
    }
    // Max-Age=0 or Expires in the past means delete — keep it with
    // `expires` set so the jar's prune step can evict it.
    Some(cookie)
}

/// Merge a newly-parsed cookie into a jar. Matches by `(name, domain,
/// path)` — same tuple the browser uses to decide replace-vs-add.
/// An already-expired cookie removes any matching entry.
pub fn upsert(jar: &mut Vec<StoredCookie>, cookie: StoredCookie) {
    let expired = cookie
        .expires
        .map(|exp| exp <= now_epoch())
        .unwrap_or(false);
    jar.retain(|c| {
        !(c.name == cookie.name && c.domain == cookie.domain && c.path == cookie.path)
    });
    if !expired {
        jar.push(cookie);
    }
}

/// Return the `name=value` pairs from `jar` that apply to this
/// request URL. Matches are filtered by domain-suffix and path-prefix;
/// expired cookies are skipped (the jar should be pruned separately,
/// but this is defensive). Empty jar yields an empty Vec.
pub fn cookies_for_url(jar: &[StoredCookie], host: &str, path: &str) -> Vec<(String, String)> {
    let host_lc = host.to_ascii_lowercase();
    let now = now_epoch();
    jar.iter()
        .filter(|c| {
            if let Some(exp) = c.expires {
                if exp <= now {
                    return false;
                }
            }
            domain_matches(&c.domain, &host_lc) && path_matches(&c.path, path)
        })
        .map(|c| (c.name.clone(), c.value.clone()))
        .collect()
}

/// Drop expired cookies from a jar (in-place). Call periodically —
/// we do so on every send so the persisted `data.json` doesn't bloat.
pub fn prune(jar: &mut Vec<StoredCookie>) {
    let now = now_epoch();
    jar.retain(|c| c.expires.map(|exp| exp > now).unwrap_or(true));
}

fn domain_matches(cookie_domain: &str, request_host: &str) -> bool {
    // Exact or suffix match (with a leading `.` boundary).
    cookie_domain == request_host
        || request_host
            .strip_suffix(cookie_domain)
            .map(|rest| rest.is_empty() || rest.ends_with('.'))
            .unwrap_or(false)
}

fn path_matches(cookie_path: &str, request_path: &str) -> bool {
    if cookie_path == "/" {
        return true;
    }
    if request_path == cookie_path {
        return true;
    }
    if let Some(rest) = request_path.strip_prefix(cookie_path) {
        // Boundary check: next char must be `/` (or already consumed).
        return rest.starts_with('/');
    }
    false
}

fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Very narrow HTTP-date parser. RFC 1123 (`Wed, 21 Oct 2026 07:28:00 GMT`)
/// is the only format we need — servers use it 99% of the time. If we
/// can't parse it, the cookie becomes a session cookie (still usable
/// for the app's lifetime). No external `chrono` dep on purpose.
fn parse_http_date(s: &str) -> Option<i64> {
    // Wed, 21 Oct 2026 07:28:00 GMT
    let s = s.trim();
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }
    let day: i64 = parts[1].parse().ok()?;
    let month = match parts[2].to_ascii_lowercase().as_str() {
        s if s.starts_with("jan") => 1,
        s if s.starts_with("feb") => 2,
        s if s.starts_with("mar") => 3,
        s if s.starts_with("apr") => 4,
        s if s.starts_with("may") => 5,
        s if s.starts_with("jun") => 6,
        s if s.starts_with("jul") => 7,
        s if s.starts_with("aug") => 8,
        s if s.starts_with("sep") => 9,
        s if s.starts_with("oct") => 10,
        s if s.starts_with("nov") => 11,
        s if s.starts_with("dec") => 12,
        _ => return None,
    };
    let year: i64 = parts[3].parse().ok()?;
    let time_parts: Vec<&str> = parts[4].split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let h: i64 = time_parts[0].parse().ok()?;
    let m: i64 = time_parts[1].parse().ok()?;
    let sec: i64 = time_parts[2].parse().ok()?;
    Some(naive_utc_to_epoch(year, month, day, h, m, sec))
}

/// Proleptic Gregorian to Unix epoch, assuming input is UTC. Good
/// for any plausible cookie expiry between 1970 and 2099.
fn naive_utc_to_epoch(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64) -> i64 {
    fn is_leap(y: i64) -> bool {
        y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
    }
    let days_in = |year: i64, month: i64| -> i64 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => if is_leap(year) { 29 } else { 28 },
            _ => 0,
        }
    };
    let mut days: i64 = 0;
    for year in 1970..y {
        days += if is_leap(year) { 366 } else { 365 };
    }
    for month in 1..mo {
        days += days_in(y, month);
    }
    days += d - 1;
    days * 86400 + h * 3600 + mi * 60 + s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let c = parse_set_cookie("sid=abc123", "api.example.com").unwrap();
        assert_eq!(c.name, "sid");
        assert_eq!(c.value, "abc123");
        assert_eq!(c.domain, "api.example.com");
        assert_eq!(c.path, "/");
    }

    #[test]
    fn parse_with_attrs() {
        let c = parse_set_cookie(
            "token=xyz; Domain=.example.com; Path=/api; Secure; HttpOnly",
            "api.example.com",
        )
        .unwrap();
        assert_eq!(c.domain, "example.com");
        assert_eq!(c.path, "/api");
        assert!(c.secure);
        assert!(c.http_only);
    }

    #[test]
    fn parse_max_age() {
        let c = parse_set_cookie("s=1; Max-Age=3600", "a.b").unwrap();
        let now = now_epoch();
        assert!(c.expires.unwrap() > now);
        assert!(c.expires.unwrap() <= now + 3601);
    }

    #[test]
    fn domain_suffix_match() {
        assert!(domain_matches("example.com", "api.example.com"));
        assert!(domain_matches("example.com", "example.com"));
        assert!(!domain_matches("example.com", "notexample.com"));
        assert!(!domain_matches("api.example.com", "example.com"));
    }

    #[test]
    fn path_prefix_match() {
        assert!(path_matches("/", "/anything"));
        assert!(path_matches("/api", "/api"));
        assert!(path_matches("/api", "/api/v1"));
        assert!(!path_matches("/api", "/apix"));
        assert!(!path_matches("/api", "/other"));
    }

    #[test]
    fn upsert_replaces_matching() {
        let mut jar = vec![];
        upsert(
            &mut jar,
            StoredCookie {
                name: "s".into(),
                value: "1".into(),
                domain: "a".into(),
                path: "/".into(),
                expires: None,
                secure: false,
                http_only: false,
            },
        );
        upsert(
            &mut jar,
            StoredCookie {
                name: "s".into(),
                value: "2".into(),
                domain: "a".into(),
                path: "/".into(),
                expires: None,
                secure: false,
                http_only: false,
            },
        );
        assert_eq!(jar.len(), 1);
        assert_eq!(jar[0].value, "2");
    }

    #[test]
    fn upsert_expired_deletes() {
        let mut jar = vec![StoredCookie {
            name: "s".into(),
            value: "1".into(),
            domain: "a".into(),
            path: "/".into(),
            expires: None,
            secure: false,
            http_only: false,
        }];
        upsert(
            &mut jar,
            StoredCookie {
                name: "s".into(),
                value: "".into(),
                domain: "a".into(),
                path: "/".into(),
                expires: Some(1),
                secure: false,
                http_only: false,
            },
        );
        assert!(jar.is_empty());
    }

    #[test]
    fn cookies_for_url_filters() {
        let jar = vec![
            StoredCookie {
                name: "a".into(),
                value: "1".into(),
                domain: "example.com".into(),
                path: "/".into(),
                expires: None,
                secure: false,
                http_only: false,
            },
            StoredCookie {
                name: "b".into(),
                value: "2".into(),
                domain: "other.com".into(),
                path: "/".into(),
                expires: None,
                secure: false,
                http_only: false,
            },
        ];
        let matched = cookies_for_url(&jar, "api.example.com", "/anything");
        assert_eq!(matched, vec![("a".to_string(), "1".to_string())]);
    }
}
