use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxArchitectureTool;

impl McpTool for CtxArchitectureTool {
    fn name(&self) -> &'static str {
        "ctx_architecture"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_architecture",
            "Graph-based architecture analysis. Actions: overview|clusters|layers|cycles|entrypoints|module.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["overview", "clusters", "layers", "cycles", "entrypoints", "module"],
                        "description": "Architecture operation (default: overview)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Used for action=module (module/file path)"
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
        let action = get_str(args, "action").unwrap_or_else(|| "overview".to_string());
        let path = get_str(args, "path");
        let format = get_str(args, "format");
        let root = ctx
            .resolved_path("root")
            .or(ctx.resolved_path("project_root"))
            .unwrap_or(&ctx.project_root);

        let result = crate::tools::ctx_architecture::handle(
            &action,
            path.as_deref(),
            root,
            format.as_deref(),
        );

        Ok(ToolOutput::simple(result))
    }
}
