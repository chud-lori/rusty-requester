use crate::io::curl;
use crate::model::{Auth, Request};
use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId};

// Syntax-highlight palette. Dark values are Monokai-ish (original);
// light values are GitHub-ish (dark ink on soft paper). Functions read
// the global active theme so response/snippet views flip automatically
// when the user toggles Settings → Theme.
fn hl_text() -> Color32 {
    if crate::theme::is_light() {
        Color32::from_rgb(31, 35, 42) // #1F232A — matches palette text
    } else {
        Color32::from_rgb(224, 226, 232)
    }
}
fn hl_string() -> Color32 {
    if crate::theme::is_light() {
        Color32::from_rgb(10, 48, 105) // #0A3069 — dark blue strings
    } else {
        Color32::from_rgb(230, 219, 116) // Monokai yellow
    }
}
fn hl_flag() -> Color32 {
    if crate::theme::is_light() {
        Color32::from_rgb(17, 99, 41) // #116329 — dark green
    } else {
        Color32::from_rgb(102, 217, 239) // Monokai cyan
    }
}
fn hl_number() -> Color32 {
    if crate::theme::is_light() {
        Color32::from_rgb(149, 56, 0) // #953800 — dark orange
    } else {
        Color32::from_rgb(174, 129, 255) // Monokai purple
    }
}
fn hl_keyword() -> Color32 {
    if crate::theme::is_light() {
        Color32::from_rgb(207, 34, 46) // #CF222E — dark red
    } else {
        Color32::from_rgb(249, 38, 114) // Monokai pink
    }
}
fn hl_comment() -> Color32 {
    if crate::theme::is_light() {
        Color32::from_rgb(110, 119, 129) // #6E7781 — mid gray
    } else {
        Color32::from_rgb(117, 113, 94) // Monokai comment grey
    }
}
fn hl_lineno() -> Color32 {
    if crate::theme::is_light() {
        Color32::from_rgb(140, 149, 159) // #8C959F — dim gray on paper
    } else {
        Color32::from_rgb(100, 105, 115)
    }
}
fn hl_json_key() -> Color32 {
    if crate::theme::is_light() {
        Color32::from_rgb(5, 80, 174) // #0550AE — medium blue keys
    } else {
        Color32::from_rgb(166, 226, 46) // Monokai pale green
    }
}

// `build_snippet_layout_job` (the with-embedded-gutter variant) was
// replaced by `build_snippet_layout_job_content_only` paired with a
// separate gutter column. Kept here only if we ever need a single-
// LayoutJob fallback; marked dead_code to keep the build quiet.
#[allow(dead_code)]
pub fn build_snippet_layout_job(text: &str, lang: SnippetLang, _wrap_width: f32) -> LayoutJob {
    let font = FontId::monospace(12.5);
    let mut job = LayoutJob::default();
    for (line_idx, line) in text.split('\n').enumerate() {
        if line_idx > 0 {
            append(&mut job, "\n", &font, hl_text());
        }
        let lineno = format!("{:>3}  ", line_idx + 1);
        append(&mut job, &lineno, &font, hl_lineno());
        highlight_line(&mut job, line, lang, &font);
    }
    job
}

/// Same as `build_snippet_layout_job` but without the line-number gutter
/// embedded into the text. Intended to be paired with a separate gutter
/// column rendered to the left of the content — that way wrapped visual
/// rows of a long logical line continue inside the content column
/// instead of snapping back to the widget's left edge and overlapping
/// the gutter.
pub fn build_snippet_layout_job_content_only(text: &str, lang: SnippetLang) -> LayoutJob {
    let font = FontId::monospace(12.5);
    let mut job = LayoutJob::default();
    for (line_idx, line) in text.split('\n').enumerate() {
        if line_idx > 0 {
            append(&mut job, "\n", &font, hl_text());
        }
        highlight_line(&mut job, line, lang, &font);
    }
    job
}

fn append(job: &mut LayoutJob, text: &str, font: &FontId, color: Color32) {
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font.clone(),
            color,
            ..Default::default()
        },
    );
}

fn highlight_line(job: &mut LayoutJob, line: &str, lang: SnippetLang, font: &FontId) {
    // HTTPie / shell — a leading `#` comment colors the whole line.
    if matches!(lang, SnippetLang::HttpieShell | SnippetLang::Curl)
        && line.trim_start().starts_with('#')
    {
        append(job, line, font, hl_comment());
        return;
    }
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut default_run_start = 0usize;

    let flush_default = |job: &mut LayoutJob, line: &str, start: usize, end: usize| {
        if end > start {
            append(job, &line[start..end], font, hl_text());
        }
    };

    while i < bytes.len() {
        let c = bytes[i];
        // String literal — quote matching, respects escapes.
        if c == b'\'' || c == b'"' {
            flush_default(job, line, default_run_start, i);
            let quote = c;
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < bytes.len() {
                i += 1; // closing quote
            }
            append(job, &line[start..i], font, hl_string());
            default_run_start = i;
            continue;
        }

        // CLI flag — `-x` or `--xxx`, only when preceded by whitespace or
        // at the very start of the line.
        if c == b'-' {
            let preceded_ok = i == 0 || bytes[i - 1].is_ascii_whitespace();
            if preceded_ok {
                let start = i;
                i += 1;
                if i < bytes.len() && bytes[i] == b'-' {
                    i += 1;
                }
                let name_start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
                {
                    i += 1;
                }
                if i > name_start {
                    flush_default(job, line, default_run_start, start);
                    append(job, &line[start..i], font, hl_flag());
                    default_run_start = i;
                    continue;
                }
                // Not a flag after all — roll back.
                i = start;
            }
        }

        // Number literal.
        if c.is_ascii_digit() {
            let prev_is_word =
                i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
            if !prev_is_word {
                flush_default(job, line, default_run_start, i);
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                append(job, &line[start..i], font, hl_number());
                default_run_start = i;
                continue;
            }
        }

        // Language keywords (Python / JS).
        if c.is_ascii_alphabetic() || c == b'_' {
            let prev_is_word =
                i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
            if !prev_is_word {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[start..i];
                if is_keyword(word, lang) {
                    flush_default(job, line, default_run_start, start);
                    append(job, word, font, hl_keyword());
                    default_run_start = i;
                }
                continue;
            }
        }

        // Advance one UTF-8 codepoint (not one byte) to stay valid.
        let rest = &line[i..];
        let step = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        i += step;
    }
    flush_default(job, line, default_run_start, bytes.len());
}

/// Build a syntax-highlighted `LayoutJob` for a JSON payload with a
/// line-number gutter. `search` (case-insensitive) paints match
/// backgrounds in accent color; empty = no search highlight.
pub fn build_json_layout_job_with_search(text: &str, search: &str) -> LayoutJob {
    let font = FontId::monospace(12.5);
    let mut job = LayoutJob::default();
    let search_lc = search.to_lowercase();
    let search_opt = if search_lc.is_empty() {
        None
    } else {
        Some(search_lc.as_str())
    };

    for (line_idx, line) in text.split('\n').enumerate() {
        if line_idx > 0 {
            append(&mut job, "\n", &font, hl_text());
        }
        let lineno = format!("{:>4}  ", line_idx + 1);
        append(&mut job, &lineno, &font, hl_lineno());
        highlight_json_line(&mut job, line, &font);
    }
    if let Some(q) = search_opt {
        apply_search_highlight(&mut job, text, q);
    }
    job
}

/// Post-pass: rewrite each section's background where its text
/// substring-matches `query` (case-insensitive). We walk the
/// accumulated sections, find match ranges in the section's own text,
/// and split sections so only matched chars get the highlight bg.
fn apply_search_highlight(job: &mut LayoutJob, _full_text: &str, query: &str) {
    let sections = std::mem::take(&mut job.sections);
    let text = job.text.clone();
    job.text.clear();
    let bg = Color32::from_rgba_unmultiplied(206, 66, 43, 120); // rust-orange highlight

    for sec in sections {
        let slice = &text[sec.byte_range.clone()];
        let slice_lc = slice.to_lowercase();
        let mut cursor = 0;
        // Find every match in this section, split accordingly.
        loop {
            let rest = &slice_lc[cursor..];
            let Some(rel) = rest.find(query) else { break };
            let match_start = cursor + rel;
            let match_end = match_start + query.len();
            if match_start > cursor {
                append_section(job, &slice[cursor..match_start], &sec.format, None);
            }
            append_section(job, &slice[match_start..match_end], &sec.format, Some(bg));
            cursor = match_end;
            if cursor >= slice_lc.len() {
                break;
            }
        }
        if cursor < slice.len() {
            append_section(job, &slice[cursor..], &sec.format, None);
        }
    }
}

fn append_section(job: &mut LayoutJob, piece: &str, base: &TextFormat, bg: Option<Color32>) {
    let mut fmt = base.clone();
    if let Some(c) = bg {
        fmt.background = c;
    }
    job.append(piece, 0.0, fmt);
}

fn highlight_json_line(job: &mut LayoutJob, line: &str, font: &FontId) {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut default_start = 0usize;

    let flush = |job: &mut LayoutJob, line: &str, start: usize, end: usize| {
        if end > start {
            append(job, &line[start..end], font, hl_text());
        }
    };

    while i < bytes.len() {
        let c = bytes[i];

        // JSON string — always double-quoted; detect whether it's a key
        // (followed by `:`) or a value string so they can take different
        // colors.
        if c == b'"' {
            flush(job, line, default_start, i);
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < bytes.len() {
                i += 1;
            }
            // Look ahead: is the next non-whitespace char `:`? That means
            // this is an object key.
            let mut peek = i;
            while peek < bytes.len() && bytes[peek].is_ascii_whitespace() {
                peek += 1;
            }
            let is_key = peek < bytes.len() && bytes[peek] == b':';
            let color = if is_key { hl_json_key() } else { hl_string() };
            append(job, &line[start..i], font, color);
            default_start = i;
            continue;
        }

        // Keyword literal: true / false / null.
        if c.is_ascii_alphabetic() {
            let prev_is_word =
                i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
            if !prev_is_word {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[start..i];
                if matches!(word, "true" | "false" | "null") {
                    flush(job, line, default_start, start);
                    append(job, word, font, hl_keyword());
                    default_start = i;
                }
                continue;
            }
        }

        // Numbers (incl. leading `-` and decimals).
        if c.is_ascii_digit() || (c == b'-' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit())
        {
            let start = i;
            if c == b'-' {
                i += 1;
            }
            while i < bytes.len()
                && (bytes[i].is_ascii_digit()
                    || bytes[i] == b'.'
                    || bytes[i] == b'e'
                    || bytes[i] == b'E'
                    || bytes[i] == b'+'
                    || bytes[i] == b'-')
            {
                i += 1;
            }
            flush(job, line, default_start, start);
            append(job, &line[start..i], font, hl_number());
            default_start = i;
            continue;
        }

        // Advance one codepoint.
        let rest = &line[i..];
        let step = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        i += step;
    }
    flush(job, line, default_start, bytes.len());
}

fn is_keyword(word: &str, lang: SnippetLang) -> bool {
    match lang {
        SnippetLang::Python => matches!(
            word,
            "import"
                | "from"
                | "as"
                | "if"
                | "else"
                | "return"
                | "print"
                | "def"
                | "None"
                | "True"
                | "False"
        ),
        SnippetLang::JavaScript => matches!(
            word,
            "const"
                | "let"
                | "var"
                | "function"
                | "return"
                | "await"
                | "async"
                | "if"
                | "else"
                | "true"
                | "false"
                | "null"
        ),
        _ => false,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SnippetLang {
    Curl,
    Python,
    JavaScript,
    HttpieShell,
}

impl SnippetLang {
    pub fn label(&self) -> &'static str {
        match self {
            SnippetLang::Curl => "cURL",
            SnippetLang::Python => "Python (requests)",
            SnippetLang::JavaScript => "JavaScript (fetch)",
            SnippetLang::HttpieShell => "HTTPie",
        }
    }
}

pub fn render_snippet(req: &Request, lang: SnippetLang) -> String {
    match lang {
        SnippetLang::Curl => curl::to_curl(req),
        SnippetLang::Python => python(req),
        SnippetLang::JavaScript => javascript(req),
        SnippetLang::HttpieShell => httpie(req),
    }
}

fn collect_send_headers(req: &Request) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = req
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| (h.key.clone(), h.value.clone()))
        .collect();
    if let Auth::Bearer { token } = &req.auth {
        if !token.is_empty() {
            out.push(("Authorization".into(), format!("Bearer {}", token)));
        }
    }
    // OAuth2 → Bearer <access_token>, mirroring the send path.
    if let Auth::OAuth2(s) = &req.auth {
        if !s.access_token.is_empty() {
            out.push(("Authorization".into(), format!("Bearer {}", s.access_token)));
        }
    }
    out
}

fn python(req: &Request) -> String {
    let mut s = String::new();
    s.push_str("import requests\n\n");
    let url = curl::build_full_url(&crate::net::ensure_url_scheme(&req.url), &req.query_params);
    s.push_str(&format!("url = {:?}\n", url));
    let headers = collect_send_headers(req);
    if !headers.is_empty() {
        s.push_str("headers = {\n");
        for (k, v) in &headers {
            s.push_str(&format!("    {:?}: {:?},\n", k, v));
        }
        s.push_str("}\n");
    } else {
        s.push_str("headers = {}\n");
    }
    let cookies: Vec<(&String, &String)> = req
        .cookies
        .iter()
        .filter(|c| c.enabled && !c.key.is_empty())
        .map(|c| (&c.key, &c.value))
        .collect();
    if !cookies.is_empty() {
        s.push_str("cookies = {\n");
        for (k, v) in &cookies {
            s.push_str(&format!("    {:?}: {:?},\n", k, v));
        }
        s.push_str("}\n");
    }
    let mut auth_arg = String::new();
    if let Auth::Basic { username, password } = &req.auth {
        if !username.is_empty() {
            auth_arg = format!(", auth=({:?}, {:?})", username, password);
        }
    }
    let cookies_arg = if cookies.is_empty() {
        ""
    } else {
        ", cookies=cookies"
    };
    let method = format!("{}", req.method).to_lowercase();
    if !req.body.is_empty() {
        s.push_str(&format!("payload = {:?}\n\n", req.body));
        s.push_str(&format!(
            "response = requests.{}(url, headers=headers{}{}, data=payload)\n",
            method, cookies_arg, auth_arg
        ));
    } else {
        s.push_str(&format!(
            "\nresponse = requests.{}(url, headers=headers{}{})\n",
            method, cookies_arg, auth_arg
        ));
    }
    s.push_str("print(response.status_code)\nprint(response.text)\n");
    s
}

fn javascript(req: &Request) -> String {
    let mut s = String::new();
    let url = curl::build_full_url(&crate::net::ensure_url_scheme(&req.url), &req.query_params);
    s.push_str(&format!("const url = {:?};\n\n", url));
    s.push_str("const options = {\n");
    s.push_str(&format!("  method: {:?},\n", req.method.to_string()));
    let headers = collect_send_headers(req);
    let cookies: Vec<String> = req
        .cookies
        .iter()
        .filter(|c| c.enabled && !c.key.is_empty())
        .map(|c| format!("{}={}", c.key, c.value))
        .collect();
    let mut header_lines: Vec<String> = headers
        .iter()
        .map(|(k, v)| format!("    {:?}: {:?}", k, v))
        .collect();
    if !cookies.is_empty() {
        header_lines.push(format!("    \"Cookie\": {:?}", cookies.join("; ")));
    }
    if !header_lines.is_empty() {
        s.push_str("  headers: {\n");
        s.push_str(&header_lines.join(",\n"));
        s.push_str(",\n  },\n");
    }
    if !req.body.is_empty() {
        s.push_str(&format!("  body: {:?},\n", req.body));
    }
    s.push_str("};\n\n");
    s.push_str("fetch(url, options)\n");
    s.push_str("  .then((res) => res.text())\n");
    s.push_str("  .then(console.log)\n");
    s.push_str("  .catch(console.error);\n");
    s
}

fn httpie(req: &Request) -> String {
    let mut parts: Vec<String> = vec!["http".into(), format!("{}", req.method)];
    let url = curl::build_full_url(&crate::net::ensure_url_scheme(&req.url), &req.query_params);
    parts.push(format!("'{}'", url.replace('\'', "'\\''")));
    for (k, v) in collect_send_headers(req) {
        parts.push(format!("'{}:{}'", k, v));
    }
    if let Auth::Basic { username, password } = &req.auth {
        if !username.is_empty() {
            parts.push(format!("--auth='{}:{}'", username, password));
        }
    }
    let cookies: Vec<String> = req
        .cookies
        .iter()
        .filter(|c| c.enabled && !c.key.is_empty())
        .map(|c| format!("{}={}", c.key, c.value))
        .collect();
    if !cookies.is_empty() {
        parts.push(format!("'Cookie:{}'", cookies.join("; ")));
    }
    let mut s = parts.join(" ");
    if !req.body.is_empty() {
        s.push_str(&format!(
            "\n# body (use --raw or pipe stdin):\n# echo {:?} | http ...",
            req.body
        ));
    }
    s
}
