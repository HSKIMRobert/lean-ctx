use std::path::Path;

use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxIndexTool;

impl McpTool for CtxIndexTool {
    fn name(&self) -> &'static str {
        "ctx_index"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_index",
            "Index orchestration. Actions: status|build|build-full.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "build", "build-full"],
                        "description": "Index action"
                    },
                    "project_root": {
                        "type": "string",
                        "description": "Project root (default: session project root)"
                    }
                },
                "required": ["action"]
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let action = get_str(args, "action")
            .ok_or_else(|| ErrorData::invalid_params("action is required", None))?;
        let root = ctx
            .resolved_path("project_root")
            .or(ctx.resolved_path("root"))
            .unwrap_or(&ctx.project_root);

        let result = crate::tools::ctx_index::handle(&action, Path::new(root));

        Ok(ToolOutput::simple(result))
    }
}
