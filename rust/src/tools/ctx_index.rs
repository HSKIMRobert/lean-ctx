use std::path::Path;

pub fn handle(action: &str, project_root: &Path) -> String {
    match action {
        "status" => {
            crate::core::index_orchestrator::status_json(project_root.to_string_lossy().as_ref())
        }
        "build" => {
            crate::core::index_orchestrator::ensure_all_background(
                project_root.to_string_lossy().as_ref(),
            );
            "started".to_string()
        }
        "build-full" => {
            // Force rebuild by deleting existing on-disk indexes first.
            let bm25 = crate::core::vector_index::BM25Index::index_file_path(project_root);
            let _ = std::fs::remove_file(&bm25);
            if let Some(dir) = crate::core::graph_index::ProjectIndex::index_dir(
                project_root.to_string_lossy().as_ref(),
            ) {
                let _ = std::fs::remove_file(dir.join("index.json"));
            }
            crate::core::index_orchestrator::ensure_all_background(
                project_root.to_string_lossy().as_ref(),
            );
            "started".to_string()
        }
        _ => "Unknown action. Use: status, build, build-full".to_string(),
    }
}
