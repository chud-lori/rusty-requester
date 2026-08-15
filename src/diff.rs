//! Minimal line-level diff — produces a unified-diff-ish view of two
//! response bodies. Good enough for "send twice, compare" on JSON /
//! text responses; not trying to be `git diff`.
//!
//! Algorithm: trim the common prefix/suffix (linear), then run a
//! longest common subsequence (LCS) over the remaining middle and
//! walk back to produce a flat list of `(Op, line)` pairs. The LCS
//! table is O(n·m) time and memory, so it is capped at
//! `LCS_MAX_LINES` per side — beyond that the middle is emitted as a
//! blanket removed/added block instead of allocating a multi-GB
//! table. For 100k+ line bodies a patience / histogram diff would be
//! finer-grained, but that's outside the 90% case and avoids the
//! extra dependency.

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

/// Above this many lines (per side, after common prefix/suffix
/// trimming) the quadratic LCS table is not built — two 20k-line
/// bodies would need a 400M-entry table (~1.6 GB). Past the cap the
/// trimmed middle is emitted as one blanket removed/added block:
/// still a valid diff, just without intra-block matching.
const LCS_MAX_LINES: usize = 5000;

/// Diff two strings as line arrays.
pub fn diff_lines(before: &str, after: &str) -> Vec<DiffLine> {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    let n = a.len();
    let m = b.len();

    // Trim the common prefix and suffix first — linear, and it lets
    // the common cases (identical bodies, one changed hunk) skip the
    // quadratic table entirely.
    let mut prefix = 0usize;
    while prefix < n && prefix < m && a[prefix] == b[prefix] {
        prefix += 1;
    }
    let mut suffix = 0usize;
    while suffix < n - prefix && suffix < m - prefix && a[n - 1 - suffix] == b[m - 1 - suffix] {
        suffix += 1;
    }

    let mut out: Vec<DiffLine> = Vec::with_capacity(n + m - prefix - suffix);
    for i in 0..prefix {
        out.push(DiffLine {
            op: Op::Same,
            text: a[i].to_string(),
            lhs: Some(i + 1),
            rhs: Some(i + 1),
        });
    }

    let mid_a = &a[prefix..n - suffix];
    let mid_b = &b[prefix..m - suffix];
    if mid_a.len() > LCS_MAX_LINES || mid_b.len() > LCS_MAX_LINES {
        // Middle too large for the quadratic table: emit it as one
        // blanket removed/added block. Coarser than a real LCS but
        // bounded in time and memory regardless of body size.
        for (k, line) in mid_a.iter().enumerate() {
            out.push(DiffLine {
                op: Op::Removed,
                text: line.to_string(),
                lhs: Some(prefix + k + 1),
                rhs: None,
            });
        }
        for (k, line) in mid_b.iter().enumerate() {
            out.push(DiffLine {
                op: Op::Added,
                text: line.to_string(),
                lhs: None,
                rhs: Some(prefix + k + 1),
            });
        }
    } else {
        lcs_diff_middle(mid_a, mid_b, prefix, &mut out);
    }

    for k in 0..suffix {
        out.push(DiffLine {
            op: Op::Same,
            text: a[n - suffix + k].to_string(),
            lhs: Some(n - suffix + k + 1),
            rhs: Some(m - suffix + k + 1),
        });
    }
    out
}

/// LCS diff over the trimmed middle. `offset` is the number of
/// trimmed prefix lines — identical on both sides by construction —
/// so `offset + i` / `offset + j` recover 1-based line numbers in
/// the full bodies.
fn lcs_diff_middle(a: &[&str], b: &[&str], offset: usize, out: &mut Vec<DiffLine>) {
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
    let mut rev: Vec<DiffLine> = Vec::with_capacity(n + m);
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            rev.push(DiffLine {
                op: Op::Same,
                text: a[i - 1].to_string(),
                lhs: Some(offset + i),
                rhs: Some(offset + j),
            });
            i -= 1;
            j -= 1;
        } else if lcs[i - 1][j] >= lcs[i][j - 1] {
            rev.push(DiffLine {
                op: Op::Removed,
                text: a[i - 1].to_string(),
                lhs: Some(offset + i),
                rhs: None,
            });
            i -= 1;
        } else {
            rev.push(DiffLine {
                op: Op::Added,
                text: b[j - 1].to_string(),
                lhs: None,
                rhs: Some(offset + j),
            });
            j -= 1;
        }
    }
    while i > 0 {
        rev.push(DiffLine {
            op: Op::Removed,
            text: a[i - 1].to_string(),
            lhs: Some(offset + i),
            rhs: None,
        });
        i -= 1;
    }
    while j > 0 {
        rev.push(DiffLine {
            op: Op::Added,
            text: b[j - 1].to_string(),
            lhs: None,
            rhs: Some(offset + j),
        });
        j -= 1;
    }
    rev.reverse();
    out.extend(rev);
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
    fn huge_identical_inputs_skip_lcs_table() {
        // 20k identical lines: the old unconditional LCS would build a
        // ~1.6 GB table here. Prefix trimming must consume everything
        // so this finishes instantly with all-Same output.
        let body: String = (0..20_000).map(|i| format!("line {}\n", i)).collect();
        let d = diff_lines(&body, &body);
        assert_eq!(d.len(), 20_000);
        assert!(d.iter().all(|l| l.op == Op::Same));
        assert_eq!(summarize(&d), (0, 0));
    }

    #[test]
    fn huge_middle_falls_back_to_blanket_diff() {
        // Shared 100-line prefix and suffix, then >LCS_MAX_LINES
        // distinct middles on both sides. The middle must be emitted
        // as blanket removed+added (no quadratic table), while the
        // trimmed prefix/suffix stay Same with correct line numbers.
        let mid = LCS_MAX_LINES + 1000;
        let mut before = String::new();
        let mut after = String::new();
        for i in 0..100 {
            let l = format!("prefix {}\n", i);
            before.push_str(&l);
            after.push_str(&l);
        }
        for i in 0..mid {
            // One line (`shared-inside`) appears in both middles — a
            // real LCS would match it and report mid-1 per side, so
            // summarize() showing full-middle counts proves the
            // blanket path ran instead.
            if i == 500 {
                before.push_str("shared-inside\n");
                after.push_str("shared-inside\n");
            } else {
                before.push_str(&format!("old {}\n", i));
                after.push_str(&format!("new {}\n", i));
            }
        }
        for i in 0..100 {
            let l = format!("suffix {}\n", i);
            before.push_str(&l);
            after.push_str(&l);
        }

        let d = diff_lines(&before, &after);
        assert_eq!(summarize(&d), (mid, mid));
        // Prefix and suffix survive as Same rows.
        assert_eq!(d[0].op, Op::Same);
        assert_eq!(d[0].lhs, Some(1));
        assert_eq!(d[0].rhs, Some(1));
        let last = d.last().unwrap();
        assert_eq!(last.op, Op::Same);
        assert_eq!(last.lhs, Some(100 + mid + 100));
        assert_eq!(last.rhs, Some(100 + mid + 100));
        // Blanket path: even the shared middle line is not matched.
        assert!(!d.iter().any(|l| l.op == Op::Same && l.text == "shared-inside"));
        // First middle row is a Removed with the right line number.
        assert_eq!(d[100].op, Op::Removed);
        assert_eq!(d[100].lhs, Some(101));
        assert_eq!(d[100].rhs, None);
    }

    #[test]
    fn small_middle_still_uses_lcs_matching() {
        // Under the cap, common lines inside the changed region must
        // still be matched (Same), not blanket-removed/added.
        let before = "start\nold 1\nshared\nold 2\nend";
        let after = "start\nnew 1\nshared\nnew 2\nend";
        let d = diff_lines(before, after);
        assert_eq!(summarize(&d), (2, 2));
        assert!(d
            .iter()
            .any(|l| l.op == Op::Same && l.text == "shared" && l.lhs == Some(3)));
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
