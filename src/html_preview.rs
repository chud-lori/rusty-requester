//! Minimal HTML → readable-text renderer for the response Body's
//! Preview mode. Deliberately NOT a full HTML renderer — no CSS,
//! no JavaScript, no real layout. The goal is making server-
//! returned HTML (error pages, login challenges, Cloudflare
//! interstitials, etc.) legible without having to squint through raw
//! tags.
//!
//! What it does:
//!   - strips `<script>` and `<style>` blocks entirely (including
//!     their content — these are actively harmful as "text")
//!   - replaces block-level tags (`<p>`, `<br>`, `<div>`, `<h1..6>`,
//!     `<li>`) with newlines so paragraphs don't collapse into one
//!     soup line
//!   - strips remaining tags
//!   - decodes the handful of HTML entities that actually show up in
//!     practice (`&amp;` `&lt;` `&gt;` `&quot;` `&apos;` `&#39;`
//!     `&nbsp;` + numeric `&#123;` and `&#xAB;`)
//!   - collapses runs of whitespace but preserves single newlines
//!
//! Zero external deps. If someone later wants real rendering they
//! can pull in `scraper` or `html5ever`, but this covers the 90%
//! case for API-returned HTML error pages.

/// Render `html` as readable plain text. Always returns something —
/// malformed input yields a best-effort output, never an error.
pub fn strip_to_text(html: &str) -> String {
    let no_scripts = strip_block_tag(html, "script");
    let no_styles = strip_block_tag(&no_scripts, "style");
    let with_newlines = replace_block_tags_with_newlines(&no_styles);
    let no_tags = strip_tags(&with_newlines);
    let decoded = decode_entities(&no_tags);
    collapse_whitespace(&decoded)
}

/// Remove the tag AND its content: `<script>...</script>` → "".
/// Case-insensitive match on the tag name. Self-closing / unclosed
/// tags are left untouched (malformed input is fine).
fn strip_block_tag(html: &str, tag: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let open_needle = format!("<{}", tag);
    let close_needle = format!("</{}>", tag);
    let open_len = open_needle.len();
    let close_len = close_needle.len();
    let lc = html.to_ascii_lowercase();
    let mut i = 0usize;
    while i < bytes.len() {
        // Look for an opening tag that matches (case-insensitively).
        if lc[i..].starts_with(&open_needle) {
            // Find the tag's `>` to see where its opening ends, then
            // find the matching close. If either's missing, bail out
            // and treat the rest as text.
            if let Some(rel_close) = lc[i..].find('>') {
                let search_from = i + rel_close + 1;
                if let Some(rel_end) = lc[search_from..].find(&close_needle) {
                    i = search_from + rel_end + close_len;
                    continue;
                }
            }
            // Orphaned opener — skip this char and keep scanning.
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    let _ = open_len;
    out
}

/// Inject a newline before certain block-level tags so readable
/// output doesn't smash paragraphs together.
fn replace_block_tags_with_newlines(html: &str) -> String {
    let block_tags = [
        "p", "br", "div", "li", "tr", "h1", "h2", "h3", "h4", "h5", "h6", "section", "article",
        "header", "footer", "nav", "pre",
    ];
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let lc = html.to_ascii_lowercase();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            for tag in &block_tags {
                let open = format!("<{}", tag);
                let close = format!("</{}", tag);
                if lc[i..].starts_with(&open) {
                    let next = i + open.len();
                    // Valid opener: next char is `>`, space, or `/` (self-closing).
                    let ok = match bytes.get(next) {
                        Some(b'>') | Some(b' ') | Some(b'/') | Some(b'\t') => true,
                        _ => false,
                    };
                    if ok {
                        out.push('\n');
                        break;
                    }
                }
                if lc[i..].starts_with(&close) {
                    let next = i + close.len();
                    let ok = matches!(bytes.get(next), Some(b'>') | Some(b' '));
                    if ok {
                        out.push('\n');
                        break;
                    }
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Strip anything between `<` and `>`. Doesn't try to understand
/// attributes or CDATA — just first-level erasure.
fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Decode the narrow set of HTML entities common in API responses.
/// Numeric (`&#65;` / `&#x41;`) entities are handled too.
fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        // Collect up to the next ';' (max 8 chars — long enough for
        // `&thinsp;` etc; refuse longer to avoid pathological input).
        let mut buf = String::new();
        let mut found_semi = false;
        for _ in 0..8 {
            match chars.peek() {
                Some(';') => {
                    chars.next();
                    found_semi = true;
                    break;
                }
                Some(_) => buf.push(chars.next().unwrap()),
                None => break,
            }
        }
        if !found_semi {
            // Not an entity — emit the `&` and the fragment verbatim.
            out.push('&');
            out.push_str(&buf);
            continue;
        }
        let replacement = match buf.as_str() {
            "amp" => "&",
            "lt" => "<",
            "gt" => ">",
            "quot" => "\"",
            "apos" => "'",
            "nbsp" => " ",
            numeric if numeric.starts_with('#') => {
                let rest = &numeric[1..];
                let code: Option<u32> =
                    if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X')) {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        rest.parse::<u32>().ok()
                    };
                if let Some(c) = code.and_then(char::from_u32) {
                    out.push(c);
                    continue;
                }
                "?"
            }
            _ => {
                // Unknown named entity — preserve verbatim so the
                // output at least tells the user something was there.
                out.push('&');
                out.push_str(&buf);
                out.push(';');
                continue;
            }
        };
        out.push_str(replacement);
    }
    out
}

/// Collapse runs of spaces/tabs to single spaces; preserve newlines
/// (we earlier injected them at block boundaries). Trims leading
/// whitespace per line and drops more-than-two consecutive blank
/// lines.
fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_streak = 0usize;
    for line in text.split('\n') {
        // Within a line, collapse tabs/runs of spaces to single spaces.
        let mut line_out = String::new();
        let mut last_space = false;
        for c in line.chars() {
            if c == ' ' || c == '\t' || c == '\r' {
                if !last_space {
                    line_out.push(' ');
                }
                last_space = true;
            } else {
                line_out.push(c);
                last_space = false;
            }
        }
        let trimmed = line_out.trim().to_string();
        if trimmed.is_empty() {
            blank_streak += 1;
            if blank_streak <= 1 {
                out.push('\n');
            }
        } else {
            blank_streak = 0;
            out.push_str(&trimmed);
            out.push('\n');
        }
    }
    // Drop trailing blank lines.
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

/// `true` if the response looks like HTML — used by the Body toolbar
/// to decide whether to surface the Preview pill. Checks the
/// Content-Type header first (authoritative); falls back to a cheap
/// body sniff.
pub fn is_html(headers: &[(String, String)], body: &str) -> bool {
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("content-type") {
            let vlc = v.to_ascii_lowercase();
            return vlc.contains("text/html") || vlc.contains("application/xhtml");
        }
    }
    // No content-type — sniff. Leading whitespace then either a
    // doctype or an html/html-ish opener.
    let trimmed = body.trim_start().to_ascii_lowercase();
    trimmed.starts_with("<!doctype html")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<head")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_script_content() {
        let out = strip_to_text("<p>hi</p><script>alert('x')</script><p>bye</p>");
        assert!(out.contains("hi"));
        assert!(out.contains("bye"));
        assert!(!out.contains("alert"));
    }

    #[test]
    fn strips_style_content() {
        let out = strip_to_text("<style>p{color:red}</style><p>visible</p>");
        assert!(!out.contains("color:red"));
        assert!(out.contains("visible"));
    }

    #[test]
    fn decodes_common_entities() {
        assert_eq!(decode_entities("&amp;&lt;&gt;"), "&<>");
        assert_eq!(decode_entities("&quot;&apos;"), "\"'");
        assert_eq!(decode_entities("&#65;&#x42;"), "AB");
        // Unknown entity passes through.
        assert_eq!(decode_entities("&zzz;"), "&zzz;");
        // Bare `&` passes through.
        assert_eq!(decode_entities("a & b"), "a & b");
    }

    #[test]
    fn block_tags_become_newlines() {
        let out = strip_to_text("<h1>Title</h1><p>Paragraph 1</p><p>Paragraph 2</p>");
        let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
        assert!(lines.contains(&"Title"));
        assert!(lines.contains(&"Paragraph 1"));
        assert!(lines.contains(&"Paragraph 2"));
    }

    #[test]
    fn detects_html_via_header() {
        let headers = vec![("Content-Type".into(), "text/html; charset=utf-8".into())];
        assert!(is_html(&headers, "anything"));
    }

    #[test]
    fn detects_html_via_sniff_when_no_header() {
        assert!(is_html(&[], "<!DOCTYPE html><html></html>"));
        assert!(is_html(&[], "  <html>"));
        assert!(!is_html(&[], r#"{"ok":true}"#));
    }

    #[test]
    fn real_world_error_page() {
        let html = r#"
            <!DOCTYPE html>
            <html><head><title>500</title><style>body{font:14px}</style></head>
            <body>
              <h1>Internal Server Error</h1>
              <p>The server encountered an error &amp; could not complete your request.</p>
              <script>console.log('snoop')</script>
            </body></html>
        "#;
        let out = strip_to_text(html);
        assert!(out.contains("Internal Server Error"));
        assert!(out.contains("error & could not complete"));
        assert!(!out.contains("console.log"));
        assert!(!out.contains("font:14px"));
    }
}
