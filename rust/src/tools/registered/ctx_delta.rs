use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxDeltaTool;

impl McpTool for CtxDeltaTool {
    fn name(&self) -> &'static str {
        "ctx_delta"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_delta",
            "Incremental diff — sends only changed lines since last read.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path" }
                },
                "required": ["path"]
            }),
        )
    }

    fn handle(
        &self,
        _args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let path = ctx
            .resolved_path("path")
            .ok_or_else(|| ErrorData::invalid_params("path is required", None))?
            .to_string();

        tokio::task::block_in_place(|| {
            let cache_lock = ctx
                .cache
                .as_ref()
                .ok_or_else(|| ErrorData::internal_error("cache not available", None))?;
            let mut cache = cache_lock.blocking_write();
            let output = crate::tools::ctx_delta::handle(&mut cache, &path);
            let original = cache.get(&path).map_or(0, |e| e.original_tokens);
            let tokens = crate::core::tokens::count_tokens(&output);
            drop(cache);

            if let Some(session_lock) = ctx.session.as_ref() {
                let mut session = session_lock.blocking_write();
                session.mark_modified(&path);
            }

            let saved = original.saturating_sub(tokens);
            Ok(ToolOutput {
                text: output,
                original_tokens: original,
                saved_tokens: saved,
                mode: Some("delta".to_string()),
                path: Some(path),
            })
        })
    }
}
