use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_int, get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxGraphDiagramTool;

impl McpTool for CtxGraphDiagramTool {
    fn name(&self) -> &'static str {
        "ctx_graph_diagram"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_graph_diagram",
            "Generate a Mermaid diagram of the dependency or call graph. Deprecated alias for ctx_graph action=diagram.",
            json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "Optional: scope to dependencies of a specific file" },
                    "depth": { "type": "integer", "description": "Max depth (default: 2)" },
                    "kind": { "type": "string", "description": "deps (file dependencies) or calls (symbol call graph)" }
                }
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let file = get_str(args, "file");
        let depth = get_int(args, "depth").map(|d| d as usize);
        let kind = get_str(args, "kind");

        let graph_output = crate::tools::ctx_graph_diagram::handle(
            file.as_deref(),
            depth,
            kind.as_deref(),
            &ctx.project_root,
        );

        let result = format!(
            "[DEPRECATED] Use ctx_graph with action='diagram' (supports same args).\n{graph_output}"
        );

        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: kind,
            path: None,
        })
    }
}
