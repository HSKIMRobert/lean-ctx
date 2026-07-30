//! Compact parsing and presentation for Cargo compiler diagnostics.

use std::cmp::Reverse;
use std::collections::BTreeMap;

macro_rules! static_regex {
    ($pattern:expr_2021) => {{
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| {
            regex::Regex::new($pattern).expect(concat!("BUG: invalid static regex: ", $pattern))
        })
    }};
}

/// Counts and grouped details extracted from rustc or Clippy output.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct DiagnosticSummary {
    /// Total error diagnostics, including repeated messages.
    pub error_count: u32,
    /// Total warning diagnostics, including repeated warning groups.
    pub warning_count: u32,
    /// Unique error messages, including error codes when available.
    pub error_messages: Vec<String>,
    /// Warning rule names with occurrence counts, sorted by count descending.
    pub warning_groups: Vec<(String, u32)>,
}

/// Separates Cargo output into its progress, diagnostic, and result sections.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct BuildPhases {
    /// Leading `Compiling`, `Checking`, and `Downloading` progress lines.
    pub compile_phase: String,
    /// Lines after progress and before Cargo reports a final result.
    pub diagnostic_phase: String,
    /// Lines beginning with `Finished` or `test result:`.
    pub result_phase: String,
}

fn error_re() -> &'static regex::Regex {
    static_regex!(r"^error(?:\[([A-Z]\d+)\])?:\s*(.+)$")
}

fn clippy_warning_re() -> &'static regex::Regex {
    static_regex!(r"^warning\[([^\]]+)\]:\s*(.+)$")
}

fn plain_warning_re() -> &'static regex::Regex {
    static_regex!(r"^warning:\s*(.+)$")
}

fn warning_summary_re() -> &'static regex::Regex {
    static_regex!(r"^(?:\d+ warnings? (?:emitted|generated)|.* generated \d+ warnings?)$")
}

/// Parses rustc and Clippy diagnostics into error messages and warning groups.
pub fn parse_and_summarize(output: &str) -> DiagnosticSummary {
    let mut summary = DiagnosticSummary::default();
    let mut warning_counts = BTreeMap::new();

    for line in output.lines() {
        let trimmed = line.trim_start();
        if let Some(captures) = error_re().captures(trimmed) {
            summary.error_count += 1;
            let message = match captures.get(1) {
                Some(code) => format!("{} {}", code.as_str(), &captures[2]),
                None => captures[2].to_string(),
            };
            if !summary.error_messages.contains(&message) {
                summary.error_messages.push(message);
            }
            continue;
        }

        if let Some(captures) = clippy_warning_re().captures(trimmed) {
            summary.warning_count += 1;
            let rule = captures[1].rsplit("::").next().unwrap_or(&captures[1]);
            *warning_counts
                .entry(normalize_warning_group(rule))
                .or_insert(0) += 1;
            continue;
        }

        if let Some(captures) = plain_warning_re().captures(trimmed) {
            let message = &captures[1];
            if warning_summary_re().is_match(message) {
                continue;
            }
            summary.warning_count += 1;
            *warning_counts
                .entry(normalize_warning_group(message))
                .or_insert(0) += 1;
        }
    }

    summary.warning_groups = warning_counts.into_iter().collect();
    summary
        .warning_groups
        .sort_unstable_by_key(|(rule, count)| (Reverse(*count), rule.clone()));
    summary
}

fn normalize_warning_group(message: &str) -> String {
    let prefix = message.split(':').next().unwrap_or(message).trim();
    let mut group = String::with_capacity(prefix.len());
    let mut previous_separator = false;

    for character in prefix.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            group.push(character.to_ascii_lowercase());
            previous_separator = false;
        } else if !previous_separator && !group.is_empty() {
            group.push('_');
            previous_separator = true;
        }
    }

    group.trim_end_matches('_').to_string()
}

/// Formats a diagnostic summary as compact, human-readable output.
pub fn format_summary(summary: &DiagnosticSummary) -> String {
    let mut sections = Vec::new();

    if summary.error_count > 0 {
        let errors = summary.error_messages.join(", ");
        let label = if summary.error_count == 1 {
            "error"
        } else {
            "errors"
        };
        if errors.is_empty() {
            sections.push(format!("{} {label}", summary.error_count));
        } else {
            sections.push(format!("{} {label}: {errors}", summary.error_count));
        }
    }

    if summary.warning_count > 0 {
        let label = if summary.warning_count == 1 {
            "warning"
        } else {
            "warnings"
        };
        let mut groups = summary
            .warning_groups
            .iter()
            .take(5)
            .map(|(rule, count)| format!("{rule} ×{count}"))
            .collect::<Vec<_>>();
        let other_count = summary.warning_groups.len().saturating_sub(5);
        if other_count > 0 {
            groups.push(format!("+{other_count} others"));
        }
        if groups.is_empty() {
            sections.push(format!("{} {label}", summary.warning_count));
        } else {
            sections.push(format!(
                "{} {label} ({})",
                summary.warning_count,
                groups.join(", ")
            ));
        }
    }

    if sections.is_empty() {
        "clean".to_string()
    } else {
        sections.join("\n")
    }
}

/// Collapses long contiguous Cargo progress runs while preserving other output.
pub fn fold_compile_progress(output: &str) -> String {
    let mut result = Vec::new();
    let lines = output.lines().collect::<Vec<_>>();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if is_ignored_progress_line(line) {
            index += 1;
            continue;
        }

        if progress_kind(line).is_none() {
            result.push(line.to_string());
            index += 1;
            continue;
        }

        let start = index;
        let mut compiled = 0;
        let mut checked = 0;
        let mut downloaded = 0;
        while index < lines.len() {
            match progress_kind(lines[index]) {
                Some(ProgressKind::Compiling) => compiled += 1,
                Some(ProgressKind::Checking) => checked += 1,
                Some(ProgressKind::Downloading) => downloaded += 1,
                None => break,
            }
            index += 1;
        }

        if compiled >= 3 || checked >= 3 || downloaded >= 3 {
            result.push(format_progress_summary(compiled, checked, downloaded));
        } else {
            result.extend(lines[start..index].iter().map(ToString::to_string));
        }
    }

    result.join("\n")
}

#[derive(Copy, Clone)]
enum ProgressKind {
    Compiling,
    Checking,
    Downloading,
}

fn progress_kind(line: &str) -> Option<ProgressKind> {
    match line.trim_start() {
        line if line.starts_with("Compiling ") => Some(ProgressKind::Compiling),
        line if line.starts_with("Checking ") => Some(ProgressKind::Checking),
        line if line.starts_with("Downloading ") => Some(ProgressKind::Downloading),
        _ => None,
    }
}

fn is_ignored_progress_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("Fresh ") || trimmed.starts_with("Blocking waiting for file lock")
}

fn format_progress_summary(compiled: u32, checked: u32, downloaded: u32) -> String {
    let mut parts = Vec::new();
    if compiled > 0 {
        parts.push(format!("compiled {compiled}"));
    }
    if checked > 0 {
        parts.push(format!("checked {checked}"));
    }
    if downloaded > 0 {
        parts.push(format!("downloaded {downloaded}"));
    }
    format!("[{} crates]", parts.join(", "))
}

/// Splits Cargo output into compile progress, diagnostics, and final result phases.
pub fn split_build_phases(output: &str) -> BuildPhases {
    let lines = output.lines().collect::<Vec<_>>();
    let compile_end = lines
        .iter()
        .position(|line| progress_kind(line).is_none())
        .unwrap_or(lines.len());
    let result_start = lines
        .iter()
        .enumerate()
        .skip(compile_end)
        .find_map(|(index, line)| is_result_line(line).then_some(index))
        .unwrap_or(lines.len());

    BuildPhases {
        compile_phase: lines[..compile_end].join("\n"),
        diagnostic_phase: lines[compile_end..result_start].join("\n"),
        result_phase: lines[result_start..].join("\n"),
    }
}

fn is_result_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("Finished ") || trimmed.starts_with("test result:")
}

#[cfg(test)]
mod tests {
    use super::{
        DiagnosticSummary, fold_compile_progress, format_summary, parse_and_summarize,
        split_build_phases,
    };

    #[test]
    fn test_clean_build() {
        let summary = parse_and_summarize(
            "Finished dev profile [unoptimized + debuginfo] target(s) in 0.21s",
        );

        assert_eq!(summary, DiagnosticSummary::default());
    }

    #[test]
    fn test_warnings_only() {
        let output = "warning: unused import: `foo`\nwarning: unused import: `bar`\nwarning: unused import: `baz`\nwarning[dead_code]: function `old` is never used\nwarning[dead_code]: function `older` is never used";
        let summary = parse_and_summarize(output);

        assert_eq!(summary.warning_count, 5);
        assert_eq!(
            summary.warning_groups,
            vec![
                ("unused_import".to_string(), 3),
                ("dead_code".to_string(), 2)
            ]
        );
    }

    #[test]
    fn test_errors_only() {
        let output = "error[E0308]: mismatched types\n  --> src/main.rs:4:5\nerror[E0308]: expected `u32`, found `String`";
        let summary = parse_and_summarize(output);

        assert_eq!(summary.error_count, 2);
        assert_eq!(
            summary.error_messages,
            vec![
                "E0308 mismatched types",
                "E0308 expected `u32`, found `String`"
            ]
        );
    }

    #[test]
    fn test_mixed() {
        let output = "warning: unused variable: `item`\nhelp: prefix it with an underscore\nerror[E0277]: the trait bound `Thing: Copy` is not satisfied";
        let summary = parse_and_summarize(output);

        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(
            summary.error_messages,
            vec!["E0277 the trait bound `Thing: Copy` is not satisfied"]
        );
        assert_eq!(
            summary.warning_groups,
            vec![("unused_variable".to_string(), 1)]
        );
    }

    #[test]
    fn test_clippy_rules() {
        let summary = parse_and_summarize(
            "warning[clippy::needless_borrow]: this expression creates a reference which is immediately dereferenced",
        );

        assert_eq!(summary.warning_count, 1);
        assert_eq!(
            summary.warning_groups,
            vec![("needless_borrow".to_string(), 1)]
        );
    }

    #[test]
    fn test_summary_format_warnings() {
        let summary = DiagnosticSummary {
            error_count: 0,
            warning_count: 21,
            error_messages: Vec::new(),
            warning_groups: vec![
                ("unused_import".to_string(), 6),
                ("dead_code".to_string(), 5),
                ("needless_borrow".to_string(), 4),
                ("unused_variable".to_string(), 3),
                ("redundant_clone".to_string(), 2),
                ("missing_docs".to_string(), 1),
            ],
        };

        assert_eq!(
            format_summary(&summary),
            "21 warnings (unused_import ×6, dead_code ×5, needless_borrow ×4, unused_variable ×3, redundant_clone ×2, +1 others)"
        );
    }

    #[test]
    fn test_fold_progress() {
        let compiling = (1..=10)
            .map(|number| format!("   Compiling crate-{number} v1.0.0"))
            .collect::<Vec<_>>();
        let checking = (1..=5)
            .map(|number| format!("    Checking crate-{number} v1.0.0"))
            .collect::<Vec<_>>();
        let mut lines = compiling;
        lines.extend(checking);
        lines.push("warning: unused import: `thing`".to_string());

        assert_eq!(
            fold_compile_progress(&lines.join("\n")),
            "[compiled 10, checked 5 crates]\nwarning: unused import: `thing`"
        );
    }

    #[test]
    fn test_split_phases() {
        let output = "   Compiling app v0.1.0\n    Checking dep v1.2.0\nwarning: unused import: `value`\n --> src/lib.rs:1:5\n    Finished dev profile [unoptimized + debuginfo] target(s) in 0.22s\ntest result: ok. 4 passed; 0 failed; 0 ignored";
        let phases = split_build_phases(output);

        assert_eq!(
            phases.compile_phase,
            "   Compiling app v0.1.0\n    Checking dep v1.2.0"
        );
        assert_eq!(
            phases.diagnostic_phase,
            "warning: unused import: `value`\n --> src/lib.rs:1:5"
        );
        assert!(phases.result_phase.starts_with("    Finished dev profile"));
    }
}
