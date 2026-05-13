use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxDedupTool;

impl McpTool for CtxDedupTool {
    fn name(&self) -> &'static str {
        "ctx_dedup"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_dedup",
            "Cross-file dedup: analyze or apply shared block references.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "analyze (default) or apply (register shared blocks for auto-dedup in ctx_read)",
                        "default": "analyze"
                    }
                }
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let action = get_str(args, "action").unwrap_or_default();
        let cache = ctx.cache.as_ref().unwrap();
        let result = if action == "apply" {
            let mut guard = tokio::task::block_in_place(|| cache.blocking_write());
            crate::tools::ctx_dedup::handle_action(&mut guard, &action)
        } else {
            let guard = tokio::task::block_in_place(|| cache.blocking_read());
            crate::tools::ctx_dedup::handle(&guard)
        };
        Ok(ToolOutput::simple(result))
    }
}
