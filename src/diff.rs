//! Minimal line-level diff — produces a unified-diff-ish view of two
//! response bodies. Good enough for "send twice, compare" on JSON /
//! text responses; not trying to be `git diff`.
//!
//! Algorithm: longest common subsequence (LCS) over line arrays, then
//! walk back to produce a flat list of `(Op, line)` pairs. O(n·m)
//! time and memory — fine for the response-size range we care about
//! (truncated at `max_body_mb`). For 100k+ line bodies a patience /
//! histogram diff would be faster, but that's outside the 90% case
//! and avoids the extra dependency.

use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Same,
    Added,
    Removed,
}

#[derive(Clone, Debug)]
pub struct DiffLine {
    pub op: Op,
    pub text: String,
    /// Line number in the `before` body, if applicable (Same / Removed).
    /// Exposed so a future gutter renderer can show 1:1 line numbers
    /// — not currently read by the UI.
    #[allow(dead_code)]
    pub lhs: Option<usize>,
    /// Line number in the `after` body, if applicable (Same / Added).
    #[allow(dead_code)]
    pub rhs: Option<usize>,
}

/// Diff two strings as line arrays.
pub fn diff_lines(before: &str, after: &str) -> Vec<DiffLine> {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    let n = a.len();
    let m = b.len();

    // Build LCS length table. Rows = a.len()+1, Cols = b.len()+1.
    // Indexed loops are clearer here than enumerate — the table is
    // 1-indexed relative to the input arrays so `i + 1` / `j + 1`
    // references read naturally.
    #[allow(clippy::needless_range_loop)]
    let mut lcs: Vec<Vec<u32>> = vec![vec![0; m + 1]; n + 1];
    #[allow(clippy::needless_range_loop)]
    for i in 0..n {
        for j in 0..m {
            lcs[i + 1][j + 1] = if a[i] == b[j] {
                lcs[i][j] + 1
            } else {
                lcs[i][j + 1].max(lcs[i + 1][j])
            };
        }
    }

    // Walk the table to emit ops in reverse, then flip.
    let mut out: Vec<DiffLine> = Vec::with_capacity(n + m);
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            out.push(DiffLine {
                op: Op::Same,
                text: a[i - 1].to_string(),
                lhs: Some(i),
                rhs: Some(j),
            });
            i -= 1;
            j -= 1;
        } else if lcs[i - 1][j] >= lcs[i][j - 1] {
            out.push(DiffLine {
                op: Op::Removed,
                text: a[i - 1].to_string(),
                lhs: Some(i),
                rhs: None,
            });
            i -= 1;
        } else {
            out.push(DiffLine {
                op: Op::Added,
                text: b[j - 1].to_string(),
                lhs: None,
                rhs: Some(j),
            });
            j -= 1;
        }
    }
    while i > 0 {
        out.push(DiffLine {
            op: Op::Removed,
            text: a[i - 1].to_string(),
            lhs: Some(i),
            rhs: None,
        });
        i -= 1;
    }
    while j > 0 {
        out.push(DiffLine {
            op: Op::Added,
            text: b[j - 1].to_string(),
            lhs: None,
            rhs: Some(j),
        });
        j -= 1;
    }
    out.reverse();
    out
}

/// Summary counts for a diff — used as the "+A -B" badge next to the
/// Diff pill.
pub fn summarize(diff: &[DiffLine]) -> (usize, usize) {
    let mut added = 0usize;
    let mut removed = 0usize;
    for l in diff {
        match l.op {
            Op::Added => added += 1,
            Op::Removed => removed += 1,
            Op::Same => {}
        }
    }
    (added, removed)
}

/// Render a diff as a flat text block (used as fallback for
/// monospace TextEdit rendering). The UI layer has a richer
/// row-based renderer; this stays around for copy-as-plaintext.
#[allow(dead_code)]
pub fn to_plain(diff: &[DiffLine]) -> String {
    let mut out = String::new();
    for l in diff {
        let prefix = match l.op {
            Op::Same => ' ',
            Op::Added => '+',
            Op::Removed => '-',
        };
        let _ = writeln!(out, "{}{}", prefix, l.text);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_inputs_all_same() {
        let d = diff_lines("a\nb\nc", "a\nb\nc");
        assert_eq!(d.len(), 3);
        assert!(d.iter().all(|l| l.op == Op::Same));
        assert_eq!(summarize(&d), (0, 0));
    }

    #[test]
    fn pure_addition() {
        let d = diff_lines("", "x\ny");
        assert_eq!(summarize(&d), (2, 0));
        assert_eq!(d[0].op, Op::Added);
        assert_eq!(d[1].op, Op::Added);
    }

    #[test]
    fn pure_removal() {
        let d = diff_lines("x\ny", "");
        assert_eq!(summarize(&d), (0, 2));
    }

    #[test]
    fn middle_change() {
        let d = diff_lines("a\nb\nc", "a\nB\nc");
        // Two variants are valid: (remove b, add B) or interleave — the
        // LCS walk produces one deterministic order. Summary is what we
        // care about.
        assert_eq!(summarize(&d), (1, 1));
        // Line numbering: `a` and `c` are Same; the changed pair keeps
        // the before/after indices.
        assert_eq!(d.first().unwrap().op, Op::Same);
        assert_eq!(d.last().unwrap().op, Op::Same);
    }

    #[test]
    fn line_numbers_track() {
        let d = diff_lines("a\nb\nc", "a\nc");
        // `a` same (1,1), `b` removed (2,-), `c` same (3,2).
        let same_a = d.iter().find(|l| l.text == "a").unwrap();
        assert_eq!(same_a.lhs, Some(1));
        assert_eq!(same_a.rhs, Some(1));
        let removed_b = d.iter().find(|l| l.text == "b").unwrap();
        assert_eq!(removed_b.op, Op::Removed);
        assert_eq!(removed_b.lhs, Some(2));
        assert_eq!(removed_b.rhs, None);
    }
}
