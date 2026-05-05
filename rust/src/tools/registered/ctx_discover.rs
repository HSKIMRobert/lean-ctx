use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_int, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxDiscoverTool;

impl McpTool for CtxDiscoverTool {
    fn name(&self) -> &'static str {
        "ctx_discover"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_discover",
            "Find missed compression opportunities in shell history.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer" }
                }
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        _ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let limit = get_int(args, "limit").unwrap_or(15) as usize;
        let history = crate::cli::load_shell_history_pub();
        let result = crate::tools::ctx_discover::discover_from_history(&history, limit);

        Ok(ToolOutput::simple(result))
    }
}
