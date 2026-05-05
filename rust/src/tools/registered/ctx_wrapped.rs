use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxWrappedTool;

impl McpTool for CtxWrappedTool {
    fn name(&self) -> &'static str {
        "ctx_wrapped"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_wrapped",
            "Session savings summary report (weekly/monthly/daily).",
            json!({
                "type": "object",
                "properties": {
                    "period": { "type": "string", "description": "week|month|day" }
                }
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        _ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let period = get_str(args, "period").unwrap_or_else(|| "week".to_string());
        let result = crate::tools::ctx_wrapped::handle(&period);
        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some(period),
            path: None,
        })
    }
}
