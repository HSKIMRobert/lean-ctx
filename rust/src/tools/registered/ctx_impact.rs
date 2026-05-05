use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_int, get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxImpactTool;

impl McpTool for CtxImpactTool {
    fn name(&self) -> &'static str {
        "ctx_impact"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_impact",
            "Graph-based impact analysis. Actions: analyze|chain|build|status.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["analyze", "chain", "build", "status"],
                        "description": "Impact operation (default: analyze)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Target file path (required for analyze). For chain: from->to spec."
                    },
                    "root": {
                        "type": "string",
                        "description": "Project root (default: .)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Max traversal depth (default: 5)"
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
        let action = get_str(args, "action").unwrap_or_else(|| "analyze".to_string());
        let path = get_str(args, "path");
        let depth = get_int(args, "depth").map(|d| d as usize);
        let format = get_str(args, "format");
        let root = ctx
            .resolved_path("root")
            .or(ctx.resolved_path("project_root"))
            .unwrap_or(&ctx.project_root);

        let result = crate::tools::ctx_impact::handle(
            &action,
            path.as_deref(),
            root,
            depth,
            format.as_deref(),
        );

        Ok(ToolOutput::simple(result))
    }
}
