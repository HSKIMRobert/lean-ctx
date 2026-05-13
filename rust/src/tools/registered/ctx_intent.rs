use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxIntentTool;

impl McpTool for CtxIntentTool {
    fn name(&self) -> &'static str {
        "ctx_intent"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_intent",
            "Structured intent input (optional) — submit compact JSON or short text; server also infers intents automatically from tool calls.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Compact JSON intent or short text" },
                    "project_root": { "type": "string", "description": "Project root directory (default: .)" }
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
        let root = ctx.resolved_path("project_root").unwrap_or(".").to_string();
        let format = get_str(args, "format");

        let cache = ctx.cache.as_ref().unwrap();
        let mut cache_guard = tokio::task::block_in_place(|| cache.blocking_write());
        let output = crate::tools::ctx_intent::handle(
            &mut cache_guard,
            &query,
            &root,
            ctx.crp_mode,
            format.as_deref(),
        );
        drop(cache_guard);

        if let Some(ref session) = ctx.session {
            let mut session_guard = tokio::task::block_in_place(|| session.blocking_write());
            session_guard.set_task(&query, Some("intent"));
        }

        Ok(ToolOutput {
            text: output,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some("semantic".to_string()),
            path: None,
        })
    }
}
