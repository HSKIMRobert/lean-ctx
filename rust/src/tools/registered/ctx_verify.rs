use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxVerifyTool;

impl McpTool for CtxVerifyTool {
    fn name(&self) -> &'static str {
        "ctx_verify"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_verify",
            "Verification observability — tool call statistics.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "description": "stats" },
                    "format": { "type": "string" }
                }
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        _ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let action = get_str(args, "action").unwrap_or_else(|| "stats".to_string());
        if action != "stats" {
            return Err(ErrorData::invalid_params(
                "unsupported action (expected: stats)",
                None,
            ));
        }
        let format = get_str(args, "format");
        let out = crate::tools::ctx_verify::handle_stats(format.as_deref())
            .map_err(|e| ErrorData::invalid_params(e, None))?;
        Ok(ToolOutput {
            text: out,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some(action),
            path: None,
        })
    }
}
