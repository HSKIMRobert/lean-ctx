//! HumanEval dataset loader (164 Python tasks, pass@1).
//!
//! Source: <https://github.com/openai/human-eval>

use serde::{Deserialize, Serialize};

/// A single HumanEval task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HumanEvalTask {
    pub task_id: String,
    pub prompt: String,
    pub canonical_solution: String,
    pub test: String,
    pub entry_point: String,
}

/// Load HumanEval tasks from an NDJSON file.
pub(crate) fn load_from_ndjson(path: &std::path::Path) -> std::io::Result<Vec<HumanEvalTask>> {
    let content = std::fs::read_to_string(path)?;
    let tasks: Vec<HumanEvalTask> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Ok(tasks)
}

/// Build the test script for a HumanEval task: solution + test harness.
pub(crate) fn build_test_script(task: &HumanEvalTask, solution: &str) -> String {
    format!(
        "{solution}\n\n{test}\n\ncheck({entry_point})\n",
        solution = solution,
        test = task.test,
        entry_point = task.entry_point,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_test_script_includes_solution_and_test() {
        let task = HumanEvalTask {
            task_id: "HumanEval/0".into(),
            prompt: "def foo():".into(),
            canonical_solution: "    return 42".into(),
            test: "def check(candidate):\n    assert candidate() == 42".into(),
            entry_point: "foo".into(),
        };
        let script = build_test_script(&task, "def foo():\n    return 42");
        assert!(script.contains("def foo()"));
        assert!(script.contains("check(foo)"));
    }
}
