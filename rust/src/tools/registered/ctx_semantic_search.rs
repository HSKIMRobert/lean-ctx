use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{
    get_bool, get_int, get_str, get_str_array, McpTool, ToolContext, ToolOutput,
};
use crate::tool_defs::tool_def;

pub struct CtxSemanticSearchTool;

impl McpTool for CtxSemanticSearchTool {
    fn name(&self) -> &'static str {
        "ctx_semantic_search"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_semantic_search",
            "Semantic code search (BM25 + optional embeddings/hybrid). action=reindex to rebuild.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language search query" },
                    "path": { "type": "string", "description": "Project root to search (default: .)" },
                    "top_k": { "type": "integer", "description": "Number of results (default: 10)" },
                    "action": { "type": "string", "description": "reindex to rebuild index" },
                    "mode": {
                        "type": "string",
                        "enum": ["bm25", "dense", "hybrid"],
                        "description": "Search mode (default: hybrid)"
                    },
                    "languages": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional: restrict to languages/extensions"
                    },
                    "path_glob": {
                        "type": "string",
                        "description": "Optional: glob over relative file paths"
                    }
                },
                "required": ["query"]
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let query = get_str(args, "query")
            .ok_or_else(|| ErrorData::invalid_params("query is required", None))?;
        let path = ctx
            .resolved_path("path")
            .unwrap_or(&ctx.project_root)
            .to_string();
        let top_k = get_int(args, "top_k").unwrap_or(10) as usize;
        let action = get_str(args, "action").unwrap_or_default();
        let mode = get_str(args, "mode");
        let languages = get_str_array(args, "languages");
        let path_glob = get_str(args, "path_glob");
        let workspace = get_bool(args, "workspace").unwrap_or(false);
        let artifacts = get_bool(args, "artifacts").unwrap_or(false);

        #[cfg(feature = "qdrant")]
        {
            let mode_effective = mode
                .as_deref()
                .unwrap_or("hybrid")
                .trim()
                .to_ascii_lowercase();
            if action != "reindex"
                && !artifacts
                && matches!(mode_effective.as_str(), "dense" | "hybrid")
                && matches!(
                    crate::core::dense_backend::DenseBackendKind::try_from_env(),
                    Ok(crate::core::dense_backend::DenseBackendKind::Qdrant)
                )
            {
                if let Some(ref session_lock) = ctx.session {
                    let value = format!(
                        "tool=ctx_semantic_search mode={mode_effective} workspace={workspace}"
                    );
                    let mut session = tokio::task::block_in_place(|| session_lock.blocking_write());
                    session.record_manual_evidence("remote:qdrant_query", Some(&value));
                }
            }
        }

        let result = if action == "reindex" {
            if artifacts {
                crate::tools::ctx_semantic_search::handle_reindex_artifacts(&path, workspace)
            } else {
                crate::tools::ctx_semantic_search::handle_reindex(&path)
            }
        } else {
            crate::tools::ctx_semantic_search::handle(
                &query,
                &path,
                top_k,
                ctx.crp_mode,
                languages.as_deref(),
                path_glob.as_deref(),
                mode.as_deref(),
                Some(workspace),
                Some(artifacts),
            )
        };

        let repeat_hint = if action == "reindex" {
            String::new()
        } else if let Some(ref autonomy) = ctx.autonomy {
            autonomy
                .track_search(&query, &path)
                .map(|h| format!("\n{h}"))
                .unwrap_or_default()
        } else {
            String::new()
        };

        Ok(ToolOutput {
            text: format!("{result}{repeat_hint}"),
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some("semantic".to_string()),
            path: None,
        })
    }
}
