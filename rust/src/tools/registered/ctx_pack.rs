use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_int, get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxPackTool;

impl McpTool for CtxPackTool {
    fn name(&self) -> &'static str {
        "ctx_pack"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_pack",
            "PR Context Pack. action=pr yields changed files, related tests, impact summary, and relevant context artifacts.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["pr"],
                        "description": "Pack action"
                    },
                    "project_root": {
                        "type": "string",
                        "description": "Project root (default: session project root)"
                    },
                    "base": {
                        "type": "string",
                        "description": "Git base ref (default: auto-detect or HEAD~1)"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["markdown", "json"],
                        "description": "Output format (default: markdown)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Impact depth (default: 3)"
                    },
                    "diff": {
                        "type": "string",
                        "description": "Optional git diff --name-status text. If omitted, computed via git."
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
        let base = get_str(args, "base");
        let format = get_str(args, "format");
        let depth = get_int(args, "depth").map(|d| d as usize);
        let diff = get_str(args, "diff");
        let project_root = ctx
            .resolved_path("project_root")
            .or(ctx.resolved_path("root"))
            .unwrap_or(&ctx.project_root);

        let result = crate::tools::ctx_pack::handle(
            &action,
            project_root,
            base.as_deref(),
            format.as_deref(),
            depth,
            diff.as_deref(),
        );

        Ok(ToolOutput::simple(result))
    }
}
