use crate::core::cache::SessionCache;
use crate::core::mode_predictor::{FileSignature, ModePredictor};
use crate::core::tokens::count_tokens;
use crate::tools::CrpMode;

pub fn select_mode(cache: &SessionCache, path: &str) -> String {
    select_mode_with_task(cache, path, None)
}

pub fn select_mode_with_task(cache: &SessionCache, path: &str, task: Option<&str>) -> String {
    let Ok(content) = std::fs::read_to_string(path) else {
        return "full".to_string();
    };

    let token_count = count_tokens(&content);
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if let Some(cached) = cache.get(path) {
        if cached.hash == compute_hash(&content) {
            return "full".to_string();
        }
        return "diff".to_string();
    }

    if token_count <= 200 {
        return "full".to_string();
    }

    if is_config_or_data(ext, path) {
        return "full".to_string();
    }

    // task mode (IB-filter) is never auto-selected — it reorders lines and breaks edits.
    // Users can still explicitly request mode: "task".

    if let Some(recommended) = intent_recommended_mode(task) {
        return recommended;
    }

    let sig = FileSignature::from_path(path, token_count);
    let predictor = ModePredictor::new();
    if let Some(predicted) = predictor.predict_best_mode(&sig) {
        return predicted;
    }

    heuristic_mode(ext, token_count)
}

/// Queries the intent engine + router for a task-aware read mode recommendation.
/// Returns `None` when there is no task, confidence is too low, or the router
/// recommends "auto" (which would recurse).
fn intent_recommended_mode(task: Option<&str>) -> Option<String> {
    let task_desc = task?;
    let classification = crate::core::intent_engine::classify(task_desc);
    if classification.confidence < 0.4 {
        return None;
    }
    let route = crate::core::intent_engine::route_intent(task_desc, &classification);
    let mode =
        crate::core::intent_router::read_mode_for_tier(route.model_tier, classification.task_type);
    if mode == "auto" {
        return None;
    }
    Some(mode)
}

fn heuristic_mode(ext: &str, token_count: usize) -> String {
    if token_count > 8000 {
        if is_code(ext) {
            return "map".to_string();
        }
        return "aggressive".to_string();
    }
    if token_count > 3000 && is_code(ext) {
        return "map".to_string();
    }
    "full".to_string()
}

pub fn handle(cache: &mut SessionCache, path: &str, crp_mode: CrpMode) -> String {
    let mode = select_mode(cache, path);
    let result = crate::tools::ctx_read::handle(cache, path, &mode, crp_mode);
    format!("[auto:{mode}] {result}")
}

fn compute_hash(content: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn is_code_ext(ext: &str) -> bool {
    is_code(ext)
}

fn is_code(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "py"
            | "go"
            | "java"
            | "c"
            | "cpp"
            | "cc"
            | "h"
            | "hpp"
            | "rb"
            | "cs"
            | "kt"
            | "swift"
            | "php"
            | "zig"
            | "ex"
            | "exs"
            | "scala"
            | "sc"
            | "dart"
            | "sh"
            | "bash"
            | "svelte"
            | "vue"
    )
}

fn is_config_or_data(ext: &str, path: &str) -> bool {
    if matches!(
        ext,
        "json" | "yaml" | "yml" | "toml" | "xml" | "ini" | "cfg" | "env" | "lock"
    ) {
        return true;
    }
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    matches!(
        name,
        "Cargo.toml"
            | "package.json"
            | "tsconfig.json"
            | "Makefile"
            | "Dockerfile"
            | "docker-compose.yml"
            | ".gitignore"
            | ".env"
            | "pyproject.toml"
            | "go.mod"
            | "build.gradle"
            | "pom.xml"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_detection() {
        assert!(is_config_or_data("json", "package.json"));
        assert!(is_config_or_data("toml", "Cargo.toml"));
        assert!(!is_config_or_data("rs", "main.rs"));
    }

    #[test]
    fn test_code_detection() {
        assert!(is_code("rs"));
        assert!(is_code("py"));
        assert!(is_code("tsx"));
        assert!(!is_code("json"));
    }

    #[test]
    fn intent_mode_for_explore_task() {
        let mode = intent_recommended_mode(Some("how does the session cache work?"));
        assert_eq!(mode, Some("map".to_string()));
    }

    #[test]
    fn intent_mode_for_fix_task() {
        let mode = intent_recommended_mode(Some("fix the bug in auth.rs"));
        assert_eq!(mode, Some("full".to_string()));
    }

    #[test]
    fn intent_mode_none_without_task() {
        assert_eq!(intent_recommended_mode(None), None);
    }

    #[test]
    fn intent_mode_none_for_low_confidence() {
        let mode = intent_recommended_mode(Some("xyz qqq"));
        assert_eq!(mode, None);
    }
}
