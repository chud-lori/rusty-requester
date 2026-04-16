//! Server-Sent Events (SSE) parser.
//!
//! Implements the line-oriented wire format from
//! https://html.spec.whatwg.org/multipage/server-sent-events.html —
//! each event is a series of `field: value` lines terminated by a
//! blank line. Multiple `data:` lines concatenate with `\n`.
//! Comment lines (lines starting with `:`) are ignored.
//!
//! Kept deliberately minimal: we only emit finished events (so a
//! `data: ...` without a trailing blank line is held in the buffer
//! until the blank arrives). Bytes are pushed in via `feed()` which
//! returns any events that completed with that chunk. Zero deps.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default)]
pub struct SseEvent {
    pub id: Option<String>,
    pub event_type: Option<String>,
    /// Multiple `data:` lines are joined with `\n`.
    pub data: String,
    pub retry_ms: Option<u64>,
    /// Unix epoch milliseconds when the event was dispatched (i.e.
    /// when the blank terminator arrived). Useful for diffing inter-
    /// event gaps in a UI log. Not yet surfaced in the Raw event log
    /// — reserved for the dedicated Events view.
    #[allow(dead_code)]
    pub timestamp_ms: u64,
}

/// Incremental parser. Call `feed(&bytes)` with each chunk from the
/// network; it returns zero or more completed events. Keep the parser
/// alive across calls so partial events buffer correctly.
pub struct SseParser {
    buf: String,
    id: Option<String>,
    event_type: Option<String>,
    data_lines: Vec<String>,
    retry: Option<u64>,
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SseParser {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            id: None,
            event_type: None,
            data_lines: Vec::new(),
            retry: None,
        }
    }

    /// Feed a chunk of bytes. Returns any events that completed with
    /// this chunk (usually zero, one, or a couple).
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        // Invalid UTF-8 is lossily replaced — SSE is text/event-stream
        // per spec so this is the right move (drop the odd byte rather
        // than error out).
        match std::str::from_utf8(bytes) {
            Ok(s) => self.buf.push_str(s),
            Err(_) => self.buf.push_str(&String::from_utf8_lossy(bytes)),
        }

        let mut events = Vec::new();
        while let Some(nl) = self.buf.find('\n') {
            let line = self.buf[..nl].trim_end_matches('\r').to_string();
            self.buf.drain(..=nl);
            if line.is_empty() {
                // Blank line — dispatch the pending event, if any.
                if !self.data_lines.is_empty() || self.event_type.is_some() || self.id.is_some() {
                    events.push(self.finalize());
                }
            } else if line.starts_with(':') {
                // Comment — ignored. Some servers send `: keep-alive`
                // every N seconds to prevent proxies from reaping.
            } else if let Some((field, value)) = split_field(&line) {
                // Per spec, a single leading space after the `:` is
                // stripped — `data: hello` → `hello`.
                let value = value.strip_prefix(' ').unwrap_or(value).to_string();
                match field {
                    "id" => self.id = Some(value),
                    "event" => self.event_type = Some(value),
                    "data" => self.data_lines.push(value),
                    "retry" => {
                        if let Ok(ms) = value.parse::<u64>() {
                            self.retry = Some(ms);
                        }
                    }
                    _ => {} // unknown field — ignore (spec-compliant).
                }
            }
        }
        events
    }

    fn finalize(&mut self) -> SseEvent {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        SseEvent {
            id: self.id.take(),
            event_type: self.event_type.take(),
            data: std::mem::take(&mut self.data_lines).join("\n"),
            retry_ms: self.retry.take(),
            timestamp_ms: ts,
        }
    }
}

fn split_field(line: &str) -> Option<(&str, &str)> {
    let idx = line.find(':')?;
    Some((&line[..idx], &line[idx + 1..]))
}

/// Pretty-print an event as a readable text block for the response
/// body view. Reproducible format so the Raw view stays scannable.
pub fn format_event(ev: &SseEvent, index: usize) -> String {
    let mut s = String::new();
    s.push_str(&format!("── event #{} ──\n", index));
    if let Some(t) = &ev.event_type {
        s.push_str(&format!("event: {}\n", t));
    }
    if let Some(id) = &ev.id {
        s.push_str(&format!("id:    {}\n", id));
    }
    if let Some(r) = ev.retry_ms {
        s.push_str(&format!("retry: {} ms\n", r));
    }
    // Try to pretty-print JSON data; fall back to raw.
    match serde_json::from_str::<serde_json::Value>(&ev.data) {
        Ok(v) => {
            let pretty = serde_json::to_string_pretty(&v).unwrap_or_else(|_| ev.data.clone());
            s.push_str("data:\n");
            for line in pretty.lines() {
                s.push_str("  ");
                s.push_str(line);
                s.push('\n');
            }
        }
        Err(_) => {
            if ev.data.is_empty() {
                s.push_str("data:  (empty)\n");
            } else if ev.data.contains('\n') {
                s.push_str("data:\n");
                for line in ev.data.lines() {
                    s.push_str("  ");
                    s.push_str(line);
                    s.push('\n');
                }
            } else {
                s.push_str(&format!("data:  {}\n", ev.data));
            }
        }
    }
    s.push('\n');
    s
}

/// `true` when a response's Content-Type indicates an SSE stream.
pub fn is_event_stream(headers: &[(String, String)]) -> bool {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.to_ascii_lowercase().contains("text/event-stream"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_event() {
        let mut p = SseParser::new();
        let events = p.feed(b"data: hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn joins_multi_line_data() {
        let mut p = SseParser::new();
        let events = p.feed(b"data: line1\ndata: line2\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn parses_event_with_fields() {
        let mut p = SseParser::new();
        let events = p.feed(b"event: update\nid: 42\ndata: {\"x\":1}\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type.as_deref(), Some("update"));
        assert_eq!(events[0].id.as_deref(), Some("42"));
        assert_eq!(events[0].data, "{\"x\":1}");
    }

    #[test]
    fn ignores_comments_and_unknown_fields() {
        let mut p = SseParser::new();
        let events = p.feed(b": keep-alive\nfoo: bar\ndata: ok\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "ok");
    }

    #[test]
    fn handles_split_across_chunks() {
        let mut p = SseParser::new();
        assert!(p.feed(b"data: par").is_empty());
        assert!(p.feed(b"tial\n").is_empty());
        let events = p.feed(b"\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "partial");
    }

    #[test]
    fn crlf_line_endings() {
        let mut p = SseParser::new();
        let events = p.feed(b"data: hi\r\n\r\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hi");
    }

    #[test]
    fn parses_retry_ms() {
        let mut p = SseParser::new();
        let events = p.feed(b"retry: 3000\ndata: x\n\n");
        assert_eq!(events[0].retry_ms, Some(3000));
    }

    #[test]
    fn detects_event_stream() {
        let h = vec![(
            "Content-Type".into(),
            "text/event-stream; charset=utf-8".into(),
        )];
        assert!(is_event_stream(&h));
        let h2 = vec![("Content-Type".into(), "application/json".into())];
        assert!(!is_event_stream(&h2));
    }
}
