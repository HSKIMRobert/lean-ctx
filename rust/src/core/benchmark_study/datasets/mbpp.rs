//! MBPP dataset loader (500 Python tasks).
//!
//! Source: <https://huggingface.co/datasets/google-research-datasets/mbpp>

use serde::{Deserialize, Serialize};

/// A single MBPP task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MbppTask {
    pub task_id: u32,
    pub text: String,
    pub code: String,
    pub test_list: Vec<String>,
}

/// Load MBPP tasks from an NDJSON file.
pub(crate) fn load_from_ndjson(path: &std::path::Path) -> std::io::Result<Vec<MbppTask>> {
    let content = std::fs::read_to_string(path)?;
    let tasks: Vec<MbppTask> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Ok(tasks)
}

/// Build the test script for an MBPP task: solution + assertions.
pub(crate) fn build_test_script(task: &MbppTask, solution: &str) -> String {
    let mut script = solution.to_string();
    script.push_str("\n\n");
    for test in &task.test_list {
        script.push_str(test);
        script.push('\n');
    }
    script
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_test_script_includes_assertions() {
        let task = MbppTask {
            task_id: 1,
            text: "Write a function to add two numbers".into(),
            code: "def add(a, b): return a + b".into(),
            test_list: vec![
                "assert add(1, 2) == 3".into(),
                "assert add(0, 0) == 0".into(),
            ],
        };
        let script = build_test_script(&task, "def add(a, b): return a + b");
        assert!(script.contains("assert add(1, 2) == 3"));
        assert!(script.contains("assert add(0, 0) == 0"));
    }
}
