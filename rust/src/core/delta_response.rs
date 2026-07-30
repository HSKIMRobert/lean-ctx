//! Delta responses for re-reads after edits (#1316).
//!
//! When a file is re-read after the agent edited it, deliver only the
//! changed lines instead of the full file. This eliminates redundant
//! delivery of unchanged content the agent already has in context.

/// A minimal unified-diff-style delta between two versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeltaResponse {
    pub path: String,
    pub hunks: Vec<Hunk>,
    pub lines_changed: usize,
    pub lines_unchanged: usize,
}

/// A single change hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hunk {
    pub old_start: usize,
    pub new_start: usize,
    pub context_before: Vec<String>,
    pub removed: Vec<String>,
    pub added: Vec<String>,
    pub context_after: Vec<String>,
}

impl DeltaResponse {
    /// Format as a compact delta for the agent.
    pub(crate) fn format(&self) -> String {
        if self.hunks.is_empty() {
            return format!("[unchanged: {} — no edits detected]", self.path);
        }

        let mut out = format!(
            "Δ {} ({} lines changed, {} unchanged)\n",
            self.path, self.lines_changed, self.lines_unchanged
        );

        for hunk in &self.hunks {
            out.push_str(&format!(
                "@@ -{},{} +{} @@\n",
                hunk.old_start,
                hunk.removed.len(),
                hunk.new_start
            ));
            for line in &hunk.context_before {
                out.push_str(&format!(" {line}\n"));
            }
            for line in &hunk.removed {
                out.push_str(&format!("-{line}\n"));
            }
            for line in &hunk.added {
                out.push_str(&format!("+{line}\n"));
            }
            for line in &hunk.context_after {
                out.push_str(&format!(" {line}\n"));
            }
        }

        out
    }

    /// Token savings from delivering delta instead of full content.
    pub(crate) fn savings_ratio(&self) -> f64 {
        let total = self.lines_changed + self.lines_unchanged;
        if total == 0 {
            return 0.0;
        }
        self.lines_unchanged as f64 / total as f64
    }
}

/// Find the next point where `old[skip_old..]` and `new[skip_new..]` agree.
/// Returns `(old_skip, new_skip)` — the number of lines to consume from each.
fn find_sync_point(old: &[&str], new: &[&str], max_look: usize) -> Option<(usize, usize)> {
    let limit_o = old.len().min(max_look);
    let limit_n = new.len().min(max_look);

    for dist in 1..=(limit_o + limit_n) {
        for skip_o in 0..=dist.min(limit_o) {
            let skip_n = dist - skip_o;
            if skip_n > limit_n {
                continue;
            }
            if skip_o < old.len() && skip_n < new.len() && old.get(skip_o) == new.get(skip_n) {
                return Some((skip_o, skip_n));
            }
        }
    }

    None
}

/// Compute a delta between `old_content` and `new_content`.
///
/// Uses a simple line-diff algorithm: identifies changed regions
/// with minimal context lines around each change.
pub(crate) fn compute_delta(
    path: &str,
    old_content: &str,
    new_content: &str,
    context_lines: usize,
) -> DeltaResponse {
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    if old_lines == new_lines {
        return DeltaResponse {
            path: path.to_string(),
            hunks: Vec::new(),
            lines_changed: 0,
            lines_unchanged: old_lines.len(),
        };
    }

    let mut hunks = Vec::new();
    let mut i = 0;
    let mut j = 0;
    let mut lines_changed = 0;

    while i < old_lines.len() || j < new_lines.len() {
        if i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
            i += 1;
            j += 1;
            continue;
        }

        let old_start = i + 1;
        let new_start = j + 1;

        let ctx_start = i.saturating_sub(context_lines);
        let context_before: Vec<String> = old_lines[ctx_start..i]
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        let mut removed = Vec::new();
        let mut added = Vec::new();

        // Find the next sync point where both sequences agree again.
        // Look ahead up to 50 lines in both directions to find a match.
        let sync = find_sync_point(&old_lines[i..], &new_lines[j..], 50);
        let (old_skip, new_skip) = sync.unwrap_or((old_lines.len() - i, new_lines.len() - j));

        for line in &old_lines[i..i + old_skip] {
            removed.push(line.to_string());
        }
        i += old_skip;

        for line in &new_lines[j..j + new_skip] {
            added.push(line.to_string());
        }
        j += new_skip;

        let ctx_end = (i + context_lines).min(old_lines.len());
        let context_after: Vec<String> = old_lines[i..ctx_end]
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        lines_changed += removed.len() + added.len();
        hunks.push(Hunk {
            old_start,
            new_start,
            context_before,
            removed,
            added,
            context_after,
        });
    }

    let lines_unchanged = old_lines.len().saturating_sub(lines_changed);

    DeltaResponse {
        path: path.to_string(),
        hunks,
        lines_changed,
        lines_unchanged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_changes_returns_empty_delta() {
        let delta = compute_delta("test.rs", "line1\nline2", "line1\nline2", 2);
        assert!(delta.hunks.is_empty());
        assert_eq!(delta.lines_unchanged, 2);
        assert_eq!(delta.format(), "[unchanged: test.rs — no edits detected]");
    }

    #[test]
    fn single_line_change() {
        let delta = compute_delta("test.rs", "a\nb\nc", "a\nB\nc", 1);
        assert_eq!(delta.hunks.len(), 1);
        assert_eq!(delta.hunks[0].removed, vec!["b"]);
        assert_eq!(delta.hunks[0].added, vec!["B"]);
    }

    #[test]
    fn format_includes_delta_marker() {
        let delta = compute_delta("src/main.rs", "old line", "new line", 0);
        let formatted = delta.format();
        assert!(formatted.contains("Δ src/main.rs"));
        assert!(formatted.contains("-old line"));
        assert!(formatted.contains("+new line"));
    }

    #[test]
    fn savings_ratio_high_for_small_change() {
        let old = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut new_lines: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
        new_lines[50] = "CHANGED LINE".to_string();
        let new = new_lines.join("\n");

        let delta = compute_delta("big.rs", &old, &new, 2);
        assert!(
            delta.savings_ratio() > 0.90,
            "savings should be >90% for 1/100 change"
        );
    }
}
