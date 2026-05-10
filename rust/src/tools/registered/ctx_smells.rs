use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxSmellsTool;

impl McpTool for CtxSmellsTool {
    fn name(&self) -> &'static str {
        "ctx_smells"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_smells",
            "Code smell detection. Actions: scan|summary|rules|file.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["scan", "summary", "rules", "file"],
                        "description": "Smell operation (default: summary)"
                    },
                    "rule": {
                        "type": "string",
                        "description": "Filter by rule name (for scan)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Filter by file path"
                    },
                    "root": {
                        "type": "string",
                        "description": "Project root (default: .)"
                    },
                    "format": {
                        "type": "string",
                        "description": "Output format"
                    }
                }
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let action = get_str(args, "action").unwrap_or_else(|| "summary".to_string());
        let rule = get_str(args, "rule");
        let path = get_str(args, "path");
        let format = get_str(args, "format");
        let root = ctx
            .resolved_path("root")
            .or(ctx.resolved_path("project_root"))
            .unwrap_or(&ctx.project_root);

        let result = crate::tools::ctx_smells::handle(
            &action,
            rule.as_deref(),
            path.as_deref(),
            root,
            format.as_deref(),
        );

        Ok(ToolOutput::simple(result))
    }
}
