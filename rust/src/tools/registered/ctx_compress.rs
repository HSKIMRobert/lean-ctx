use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_bool, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxCompressTool;

impl McpTool for CtxCompressTool {
    fn name(&self) -> &'static str {
        "ctx_compress"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_compress",
            "Context checkpoint for long conversations.",
            json!({
                "type": "object",
                "properties": {
                    "include_signatures": { "type": "boolean", "description": "Include signatures (default: true)" }
                }
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let include_sigs = get_bool(args, "include_signatures").unwrap_or(true);
        let cache = ctx.cache.as_ref().unwrap();
        let guard = tokio::task::block_in_place(|| cache.blocking_read());
        let result = crate::tools::ctx_compress::handle(&guard, include_sigs, ctx.crp_mode);
        Ok(ToolOutput::simple(result))
    }
}
