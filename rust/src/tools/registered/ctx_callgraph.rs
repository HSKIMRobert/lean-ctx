use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxCallgraphTool;

impl McpTool for CtxCallgraphTool {
    fn name(&self) -> &'static str {
        "ctx_callgraph"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_callgraph",
            "Unified call graph query. direction=callers|callees for a symbol. Returns file/symbol/line edges.",
            json!({
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "Symbol name to inspect" },
                    "direction": { "type": "string", "description": "callers|callees (default: callers)" },
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
        let direction = get_str(args, "direction").unwrap_or_else(|| "callers".to_string());
        let file = get_str(args, "file");

        let result = crate::tools::ctx_callgraph::handle(
            &symbol,
            file.as_deref(),
            &ctx.project_root,
            &direction,
        );

        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some(direction),
            path: None,
        })
    }
}
