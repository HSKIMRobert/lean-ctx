use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_int, get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxReviewTool;

impl McpTool for CtxReviewTool {
    fn name(&self) -> &'static str {
        "ctx_review"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_review",
            "Automated code review: combines impact analysis, caller tracking, and test discovery. \
             Actions: review (single file), diff-review (from git diff), checklist (structured review questions).",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["review", "diff-review", "checklist"],
                        "description": "Review action"
                    },
                    "path": {
                        "type": "string",
                        "description": "File path to review (or git diff text for diff-review)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Impact analysis depth (default: 3)"
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
        let path = get_str(args, "path");
        let depth = get_int(args, "depth").map(|d| d as usize);
        let project_root = ctx
            .resolved_path("project_root")
            .or(ctx.resolved_path("root"))
            .unwrap_or(&ctx.project_root);

        let result =
            crate::tools::ctx_review::handle(&action, path.as_deref(), project_root, depth);

        Ok(ToolOutput::simple(result))
    }
}
