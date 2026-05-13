use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxContextTool;

impl McpTool for CtxContextTool {
    fn name(&self) -> &'static str {
        "ctx_context"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_context",
            "Session context overview — cached files, seen files, session state.",
            json!({
                "type": "object",
                "properties": {}
            }),
        )
    }

    fn handle(
        &self,
        _args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let cache = ctx.cache.as_ref().unwrap();
        let guard = tokio::task::block_in_place(|| cache.blocking_read());
        let turn = ctx
            .call_count
            .as_ref()
            .map_or(0, |c| c.load(std::sync::atomic::Ordering::Relaxed));
        let result = crate::tools::ctx_context::handle_status(&guard, turn, ctx.crp_mode);
        Ok(ToolOutput::simple(result))
    }
}
