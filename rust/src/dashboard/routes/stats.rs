pub fn handle(
    path: &str,
    _query_str: &str,
    _method: &str,
    _body: &str,
) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/stats" => {
            let store = crate::core::stats::load();
            let json = serde_json::to_string(&store).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/gain" => {
            let env_model = std::env::var("LEAN_CTX_MODEL")
                .or_else(|_| std::env::var("LCTX_MODEL"))
                .ok();
            let engine = crate::core::gain::GainEngine::load();
            let payload = serde_json::json!({
                "summary": engine.summary(env_model.as_deref()),
                "tasks": engine.task_breakdown(),
                "heatmap": engine.heatmap_gains(20),
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/pulse" => {
            let stats_path = crate::core::data_dir::lean_ctx_data_dir()
                .map(|d| d.join("stats.json"))
                .unwrap_or_default();
            let meta = std::fs::metadata(&stats_path).ok();
            let size = meta.as_ref().map_or(0, std::fs::Metadata::len);
            let mtime = meta
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            use md5::Digest;
            let hash = format!(
                "{:x}",
                md5::Md5::digest(format!("{size}-{mtime}").as_bytes())
            );
            let json = format!(r#"{{"hash":"{hash}","ts":{mtime}}}"#);
            Some(("200 OK", "application/json", json))
        }
        "/api/pipeline-stats" => {
            let stats = crate::core::pipeline::PipelineStats::load();
            let json = serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        _ => None,
    }
}
