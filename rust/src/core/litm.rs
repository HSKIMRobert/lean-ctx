use crate::core::session::SessionState;

/// LITM-aware positioning constants based on Liu et al., 2023.
/// LLMs have a U-shaped attention curve:
///   - Begin (alpha ~= 0.9): highest recall
///   - Middle (beta ~= 0.55): lowest recall
///   - End (gamma ~= 0.85): high recall
///
/// CCP places critical information at begin and end positions,
/// eliminating the lossy middle entirely.
const _ALPHA: f64 = 0.9;
const _BETA: f64 = 0.55;
const _GAMMA: f64 = 0.85;

#[allow(dead_code)]
pub struct PositionedOutput {
    pub begin_block: String,
    pub end_block: String,
}

/// Sorts session state fields by attention priority:
///   P1 (begin): task, decisions, project topology, file refs
///   P2 (end): recent findings, test results, next steps
///   P3 (dropped): old completed tasks, historical reads beyond limit
pub fn position_optimize(session: &SessionState) -> PositionedOutput {
    let mut begin_lines = Vec::new();
    let mut end_lines = Vec::new();

    begin_lines.push(format!(
        "ACTIVE SESSION v{} | {} calls | {} tok saved",
        session.version, session.stats.total_tool_calls, session.stats.total_tokens_saved
    ));

    if let Some(ref task) = session.task {
        let pct = task
            .progress_pct
            .map_or(String::new(), |p| format!(" [{p}%]"));
        begin_lines.push(format!("Task: {}{pct}", task.description));
    }

    if let Some(ref root) = session.project_root {
        begin_lines.push(format!("Root: {root}"));
    }

    if !session.decisions.is_empty() {
        let items: Vec<&str> = session
            .decisions
            .iter()
            .rev()
            .take(5)
            .map(|d| d.summary.as_str())
            .collect();
        begin_lines.push(format!("Decisions: {}", items.join(" | ")));
    }

    if !session.files_touched.is_empty() {
        let items: Vec<String> = session
            .files_touched
            .iter()
            .rev()
            .take(15)
            .map(|f| {
                let r = f.file_ref.as_deref().unwrap_or("?");
                let status = if f.modified { "mod" } else { &f.last_mode };
                format!("{r}={} [{status}]", short_path(&f.path))
            })
            .collect();
        begin_lines.push(format!("Files: {}", items.join(" ")));
    }

    if !session.findings.is_empty() {
        let items: Vec<String> = session
            .findings
            .iter()
            .rev()
            .take(5)
            .map(|f| match (&f.file, f.line) {
                (Some(file), Some(line)) => format!("{}:{line} — {}", short_path(file), f.summary),
                (Some(file), None) => format!("{} — {}", short_path(file), f.summary),
                _ => f.summary.clone(),
            })
            .collect();
        end_lines.push(format!("Findings: {}", items.join(" | ")));
    }

    if let Some(ref tests) = session.test_results {
        let status = if tests.failed > 0 { "FAIL" } else { "PASS" };
        end_lines.push(format!(
            "Tests [{status}]: {}/{} ({})",
            tests.passed, tests.total, tests.command
        ));
    }

    if !session.next_steps.is_empty() {
        end_lines.push(format!("Next: {}", session.next_steps.join(" → ")));
    }

    PositionedOutput {
        begin_block: begin_lines.join("\n"),
        end_block: end_lines.join("\n"),
    }
}

#[allow(dead_code)]
/// Compute the theoretical LITM efficiency for a given context layout.
/// Returns (efficiency_without_ccp, efficiency_with_ccp) as percentages.
pub fn compute_litm_efficiency(
    begin_tokens: usize,
    middle_tokens: usize,
    end_tokens: usize,
    ccp_begin_tokens: usize,
    ccp_end_tokens: usize,
) -> (f64, f64) {
    let total_without = (begin_tokens + middle_tokens + end_tokens) as f64;
    let effective_without =
        _ALPHA * begin_tokens as f64 + _BETA * middle_tokens as f64 + _GAMMA * end_tokens as f64;

    let total_with = (ccp_begin_tokens + ccp_end_tokens) as f64;
    let effective_with = _ALPHA * ccp_begin_tokens as f64 + _GAMMA * ccp_end_tokens as f64;

    let eff_without = if total_without > 0.0 {
        effective_without / total_without * 100.0
    } else {
        0.0
    };
    let eff_with = if total_with > 0.0 {
        effective_with / total_with * 100.0
    } else {
        0.0
    };

    (eff_without, eff_with)
}

fn short_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 2 {
        return path.to_string();
    }
    parts.last().copied().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn litm_efficiency_without_ccp_lower() {
        let (eff_without, eff_with) = compute_litm_efficiency(100, 500, 100, 300, 200);
        assert!(
            eff_with > eff_without,
            "CCP should improve LITM efficiency: without={eff_without:.1}%, with={eff_with:.1}%"
        );
    }

    #[test]
    fn litm_efficiency_zero_tokens() {
        let (eff_without, eff_with) = compute_litm_efficiency(0, 0, 0, 0, 0);
        assert_eq!(eff_without, 0.0);
        assert_eq!(eff_with, 0.0);
    }

    #[test]
    fn litm_all_at_begin_is_alpha() {
        let (_, eff_with) = compute_litm_efficiency(0, 0, 0, 100, 0);
        assert!((eff_with - 90.0).abs() < 0.1, "all begin should be ~90%");
    }

    #[test]
    fn litm_all_at_end_is_gamma() {
        let (_, eff_with) = compute_litm_efficiency(0, 0, 0, 0, 100);
        assert!((eff_with - 85.0).abs() < 0.1, "all end should be ~85%");
    }

    #[test]
    fn litm_middle_heavy_is_worst() {
        let (eff_middle, _) = compute_litm_efficiency(10, 1000, 10, 0, 0);
        let (eff_balanced, _) = compute_litm_efficiency(500, 20, 500, 0, 0);
        assert!(
            eff_balanced > eff_middle,
            "middle-heavy should be less efficient"
        );
    }

    #[test]
    fn short_path_simple() {
        assert_eq!(short_path("file.rs"), "file.rs");
        assert_eq!(short_path("src/file.rs"), "src/file.rs");
        assert_eq!(short_path("a/b/c/file.rs"), "file.rs");
    }
}
