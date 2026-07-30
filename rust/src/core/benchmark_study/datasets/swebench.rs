//! SWE-bench Verified dataset loader (500 real GitHub issues).
//!
//! Source: <https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified>

use serde::{Deserialize, Serialize};

/// A single SWE-bench instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SweBenchInstance {
    pub instance_id: String,
    pub repo: String,
    pub base_commit: String,
    pub problem_statement: String,
    pub patch: String,
    pub test_patch: String,
    #[serde(default)]
    pub hints_text: String,
}

/// Load SWE-bench instances from an NDJSON file.
pub(crate) fn load_from_ndjson(path: &std::path::Path) -> std::io::Result<Vec<SweBenchInstance>> {
    let content = std::fs::read_to_string(path)?;
    let instances: Vec<SweBenchInstance> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Ok(instances)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_returns_empty_vec() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("empty.ndjson");
        std::fs::write(&path, "").unwrap();
        let result = load_from_ndjson(&path).unwrap();
        assert!(result.is_empty());
    }
}
