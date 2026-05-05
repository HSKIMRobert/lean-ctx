use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxAnalyzeTool;

impl McpTool for CtxAnalyzeTool {
    fn name(&self) -> &'static str {
        "ctx_analyze"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_analyze",
            "Entropy analysis — recommends optimal compression mode for a file.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
        )
    }

    fn handle(
        &self,
        _args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let path = ctx
            .resolved_path("path")
            .ok_or_else(|| ErrorData::invalid_params("path is required", None))?
            .to_string();

        let result = crate::tools::ctx_analyze::handle(&path, crate::tools::CrpMode::effective());

        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: None,
            path: Some(path),
        })
    }
}
