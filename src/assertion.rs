//! Post-response assertion evaluator. Given a `ResponseAssertion`, a
//! response status line, response body, and response headers, decides
//! whether the assertion passes and produces a human-readable reason
//! if it fails. No regex crate — we hand-roll a tiny substring-based
//! matcher for the `Matches` operator (see `regex_match`). If you ever
//! need full PCRE, swap this out for the `regex` crate.

use crate::extract::eval_body_path;
use crate::model::{AssertionOp, AssertionResult, AssertionSource, ResponseAssertion};

/// Run a single assertion. Never panics — bad input yields
/// `AssertionResult::Error` with a short explanation.
pub fn evaluate(
    assertion: &ResponseAssertion,
    status: &str,
    body: &str,
    headers: &[(String, String)],
) -> AssertionResult {
    // Short-circuit empty expressions for Body / Header — we can't
    // look anything up with nothing.
    let expr = assertion.expression.trim();
    let actual = match assertion.source {
        AssertionSource::Status => {
            // Status text looks like "200 OK" — take the leading code.
            status.split_whitespace().next().map(|s| s.to_string())
        }
        AssertionSource::Header => {
            if expr.is_empty() {
                return AssertionResult::Error("header name is empty".into());
            }
            headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(expr))
                .map(|(_, v)| v.clone())
        }
        AssertionSource::Body => {
            if expr.is_empty() {
                return AssertionResult::Error("body path is empty".into());
            }
            eval_body_path(body, expr)
        }
    };

    // `Exists` is a pure presence check — no comparison to `expected`.
    if matches!(assertion.op, AssertionOp::Exists) {
        return match actual {
            Some(_) => AssertionResult::Pass,
            None => AssertionResult::Fail(format!("{} not found", source_label(assertion))),
        };
    }

    let actual = match actual {
        Some(v) => v,
        None => return AssertionResult::Fail(format!("{} not found", source_label(assertion))),
    };
    let expected = assertion.expected.trim();

    match assertion.op {
        AssertionOp::Equals => {
            if actual == expected {
                AssertionResult::Pass
            } else {
                AssertionResult::Fail(format!("got {:?}, expected {:?}", actual, expected))
            }
        }
        AssertionOp::NotEquals => {
            if actual != expected {
                AssertionResult::Pass
            } else {
                AssertionResult::Fail(format!("both sides are {:?}", actual))
            }
        }
        AssertionOp::Contains => {
            if actual.contains(expected) {
                AssertionResult::Pass
            } else {
                AssertionResult::Fail(format!("{:?} does not contain {:?}", actual, expected))
            }
        }
        AssertionOp::Matches => match regex_match(expected, &actual) {
            Ok(true) => AssertionResult::Pass,
            Ok(false) => {
                AssertionResult::Fail(format!("{:?} does not match /{}/", actual, expected))
            }
            Err(e) => AssertionResult::Error(e),
        },
        AssertionOp::Exists => unreachable!(),
        AssertionOp::GreaterThan => compare_numeric(&actual, expected, |a, b| a > b),
        AssertionOp::LessThan => compare_numeric(&actual, expected, |a, b| a < b),
    }
}

fn source_label(a: &ResponseAssertion) -> String {
    match a.source {
        AssertionSource::Status => "status".to_string(),
        AssertionSource::Header => format!("header `{}`", a.expression.trim()),
        AssertionSource::Body => format!("body path `{}`", a.expression.trim()),
    }
}

fn compare_numeric(actual: &str, expected: &str, op: fn(f64, f64) -> bool) -> AssertionResult {
    let Ok(a) = actual.parse::<f64>() else {
        return AssertionResult::Error(format!("{:?} is not a number", actual));
    };
    let Ok(b) = expected.parse::<f64>() else {
        return AssertionResult::Error(format!("{:?} is not a number", expected));
    };
    if op(a, b) {
        AssertionResult::Pass
    } else {
        AssertionResult::Fail(format!("{} vs {}", a, b))
    }
}

/// Minimal glob-ish matcher. Supports `.` (any char), `.*` (any run),
/// `^` (anchor start), `$` (anchor end), literal characters. Good
/// enough for "status is 2\d\d" and "body contains hello.*world".
/// Intentionally NOT a full regex — if you need lookahead or
/// backreferences, add the `regex` crate.
fn regex_match(pattern: &str, input: &str) -> Result<bool, String> {
    let anchor_start = pattern.starts_with('^');
    let anchor_end = pattern.ends_with('$') && !pattern.ends_with("\\$");
    let pat: &str =
        &pattern[if anchor_start { 1 } else { 0 }..pattern.len() - if anchor_end { 1 } else { 0 }];
    let tokens = tokenize(pat)?;

    let input_bytes = input.as_bytes();
    if anchor_start {
        return Ok(try_match(&tokens, input_bytes, 0)
            .is_some_and(|end| !anchor_end || end == input_bytes.len()));
    }
    for start in 0..=input_bytes.len() {
        if let Some(end) = try_match(&tokens, input_bytes, start) {
            if !anchor_end || end == input_bytes.len() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[derive(Debug)]
enum Tok {
    /// Literal byte.
    Lit(u8),
    /// Any single non-newline byte (`.`).
    AnyOne,
    /// Zero or more of any byte (`.*`).
    AnyRun,
}

fn tokenize(pattern: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let bytes = pattern.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'.' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                out.push(Tok::AnyRun);
                i += 2;
            }
            b'.' => {
                out.push(Tok::AnyOne);
                i += 1;
            }
            b'\\' if i + 1 < bytes.len() => {
                out.push(Tok::Lit(bytes[i + 1]));
                i += 2;
            }
            c => {
                out.push(Tok::Lit(c));
                i += 1;
            }
        }
    }
    Ok(out)
}

/// Try to match `tokens` starting at position `pos` in `input`. Returns
/// the end index past the match, or `None` if no match. `AnyRun` is
/// greedy with backtracking.
fn try_match(tokens: &[Tok], input: &[u8], pos: usize) -> Option<usize> {
    match tokens.first() {
        None => Some(pos),
        Some(Tok::Lit(c)) => {
            if pos < input.len() && input[pos] == *c {
                try_match(&tokens[1..], input, pos + 1)
            } else {
                None
            }
        }
        Some(Tok::AnyOne) => {
            if pos < input.len() && input[pos] != b'\n' {
                try_match(&tokens[1..], input, pos + 1)
            } else {
                None
            }
        }
        Some(Tok::AnyRun) => {
            // Greedy: try longest first, backtrack on failure.
            let mut end = input.len();
            loop {
                if let Some(p) = try_match(&tokens[1..], input, end) {
                    return Some(p);
                }
                if end <= pos {
                    return None;
                }
                end -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AssertionOp, AssertionSource};

    fn a(
        source: AssertionSource,
        expr: &str,
        op: AssertionOp,
        expected: &str,
    ) -> ResponseAssertion {
        ResponseAssertion {
            enabled: true,
            source,
            expression: expr.to_string(),
            op,
            expected: expected.to_string(),
        }
    }

    #[test]
    fn status_equals_pass() {
        let r = evaluate(
            &a(AssertionSource::Status, "", AssertionOp::Equals, "200"),
            "200 OK",
            "",
            &[],
        );
        assert_eq!(r, AssertionResult::Pass);
    }

    #[test]
    fn status_equals_fail() {
        let r = evaluate(
            &a(AssertionSource::Status, "", AssertionOp::Equals, "200"),
            "500 Internal Server Error",
            "",
            &[],
        );
        assert!(matches!(r, AssertionResult::Fail(_)));
    }

    #[test]
    fn header_exists() {
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];
        let r = evaluate(
            &a(
                AssertionSource::Header,
                "content-type",
                AssertionOp::Exists,
                "",
            ),
            "200 OK",
            "",
            &headers,
        );
        assert_eq!(r, AssertionResult::Pass);
    }

    #[test]
    fn body_path_equals() {
        let r = evaluate(
            &a(
                AssertionSource::Body,
                "data.token",
                AssertionOp::Equals,
                "abc",
            ),
            "200 OK",
            r#"{"data":{"token":"abc"}}"#,
            &[],
        );
        assert_eq!(r, AssertionResult::Pass);
    }

    #[test]
    fn body_contains() {
        let r = evaluate(
            &a(AssertionSource::Body, "msg", AssertionOp::Contains, "hello"),
            "200 OK",
            r#"{"msg":"hello world"}"#,
            &[],
        );
        assert_eq!(r, AssertionResult::Pass);
    }

    #[test]
    fn status_greater_than() {
        let r = evaluate(
            &a(AssertionSource::Status, "", AssertionOp::GreaterThan, "199"),
            "200 OK",
            "",
            &[],
        );
        assert_eq!(r, AssertionResult::Pass);
    }

    #[test]
    fn matches_simple_pattern() {
        assert!(regex_match("hello.*world", "hello crazy world").unwrap());
        assert!(regex_match("^2..$", "200").unwrap());
        assert!(!regex_match("^2..$", "500").unwrap());
        assert!(regex_match("foo", "foobar").unwrap());
        assert!(!regex_match("^foo$", "foobar").unwrap());
    }
}
