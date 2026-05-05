use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxOutlineTool;

impl McpTool for CtxOutlineTool {
    fn name(&self) -> &'static str {
        "ctx_outline"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_outline",
            "List all symbols in a file (functions, structs, classes, methods) with signatures. \
Much fewer tokens than reading the full file.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "kind": { "type": "string", "description": "Optional filter: fn|struct|class|all" }
                },
                "required": ["path"]
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let path = ctx
            .resolved_path("path")
            .ok_or_else(|| ErrorData::invalid_params("path is required", None))?
            .to_string();
        let kind = get_str(args, "kind");

        let (result, original) = crate::tools::ctx_outline::handle(&path, kind.as_deref());
        let sent = crate::core::tokens::count_tokens(&result);
        let saved = original.saturating_sub(sent);

        Ok(ToolOutput {
            text: result,
            original_tokens: original,
            saved_tokens: saved,
            mode: kind,
            path: Some(path),
        })
    }
}
