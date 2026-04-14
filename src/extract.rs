//! Post-response value extraction. A tiny expression evaluator that
//! walks a dot-and-bracket path into a JSON value (e.g. `data.token`
//! or `items[0].id`) and returns the leaf as a string. No external
//! JSONPath dependency — 90% of the value in 50 lines.

use serde_json::Value;

/// Evaluate a dot/bracket path against a JSON body. Returns `None` if
/// the body isn't valid JSON, or the path misses at any step. Leaves
/// are coerced to their natural string form (strings without quotes,
/// scalars via `to_string()`, objects/arrays serialized).
pub fn eval_body_path(body: &str, path: &str) -> Option<String> {
    let root: Value = serde_json::from_str(body).ok()?;
    let leaf = walk_path(&root, path)?;
    Some(value_to_plain_string(&leaf))
}

fn walk_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = root;
    for seg in tokenize_path(path) {
        cur = match seg {
            PathSeg::Key(k) => cur.get(&k)?,
            PathSeg::Index(i) => cur.get(i)?,
        };
    }
    Some(cur)
}

#[derive(Debug, PartialEq)]
enum PathSeg {
    Key(String),
    Index(usize),
}

fn tokenize_path(path: &str) -> Vec<PathSeg> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut chars = path.chars().peekable();
    // Optional leading `$` (JSONPath-style) — accept and ignore.
    if matches!(chars.peek(), Some('$')) {
        chars.next();
        if matches!(chars.peek(), Some('.')) {
            chars.next();
        }
    }
    while let Some(c) = chars.next() {
        match c {
            '.' => {
                if !buf.is_empty() {
                    out.push(PathSeg::Key(std::mem::take(&mut buf)));
                }
            }
            '[' => {
                if !buf.is_empty() {
                    out.push(PathSeg::Key(std::mem::take(&mut buf)));
                }
                let mut idx = String::new();
                for ic in chars.by_ref() {
                    if ic == ']' {
                        break;
                    }
                    idx.push(ic);
                }
                if let Ok(n) = idx.trim().parse::<usize>() {
                    out.push(PathSeg::Index(n));
                }
            }
            _ => buf.push(c),
        }
    }
    if !buf.is_empty() {
        out.push(PathSeg::Key(buf));
    }
    out
}

fn value_to_plain_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_path() {
        let body = r#"{"data": {"token": "abc"}}"#;
        assert_eq!(eval_body_path(body, "data.token"), Some("abc".into()));
    }

    #[test]
    fn bracket_index() {
        let body = r#"{"items": [{"id": 42}, {"id": 7}]}"#;
        assert_eq!(eval_body_path(body, "items[0].id"), Some("42".into()));
        assert_eq!(eval_body_path(body, "items[1].id"), Some("7".into()));
    }

    #[test]
    fn jsonpath_root_accepted() {
        let body = r#"{"x": 1}"#;
        assert_eq!(eval_body_path(body, "$.x"), Some("1".into()));
    }

    #[test]
    fn miss_returns_none() {
        let body = r#"{"a": 1}"#;
        assert_eq!(eval_body_path(body, "b"), None);
        assert_eq!(eval_body_path(body, "a.b"), None);
    }

    #[test]
    fn scalar_root() {
        assert_eq!(eval_body_path(r#""hello""#, ""), Some("hello".into()));
    }
}
