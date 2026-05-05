use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxCallersTool;

impl McpTool for CtxCallersTool {
    fn name(&self) -> &'static str {
        "ctx_callers"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_callers",
            "Find all symbols that call a given function/method. Deprecated alias for ctx_callgraph direction=callers.",
            json!({
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "Symbol name to find callers of" },
                    "file": { "type": "string", "description": "Optional: scope to a specific file" }
                },
                "required": ["symbol"]
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let symbol = get_str(args, "symbol")
            .ok_or_else(|| ErrorData::invalid_params("symbol is required", None))?;
        let file = get_str(args, "file");

        let result = crate::tools::ctx_callers::handle(&symbol, file.as_deref(), &ctx.project_root);

        Ok(ToolOutput::simple(result))
    }
}
