use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_int, get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxGraphTool;

impl McpTool for CtxGraphTool {
    fn name(&self) -> &'static str {
        "ctx_graph"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_graph",
            "Unified code graph. Actions: build (index), related (connected files), symbol (def/usages), \
impact (blast radius), status (stats), enrich (add commits+tests+knowledge), context (task-based query), diagram (Mermaid deps/calls).",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["build", "related", "symbol", "impact", "status", "enrich", "context", "diagram"],
                        "description": "Graph operation"
                    },
                    "path": {
                        "type": "string",
                        "description": "File path (related/impact) or file::symbol_name (symbol)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Optional depth for action=diagram (default: 2)"
                    },
                    "kind": {
                        "type": "string",
                        "description": "Optional kind for action=diagram: deps|calls"
                    },
                    "project_root": {
                        "type": "string",
                        "description": "Project root directory (default: .)"
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

        // For diagram action, pass the raw path; for others, use the resolved path.
        let path = if action == "diagram" {
            get_str(args, "path")
        } else {
            ctx.resolved_path("path").map(String::from)
        };

        let root = ctx
            .resolved_path("project_root")
            .unwrap_or(&ctx.project_root)
            .to_string();
        let depth = get_int(args, "depth").map(|d| d as usize);
        let kind = get_str(args, "kind");

        let cache = ctx.cache.as_ref().unwrap();
        let mut guard = tokio::task::block_in_place(|| cache.blocking_write());
        let result = crate::tools::ctx_graph::handle(
            &action,
            path.as_deref(),
            &root,
            &mut guard,
            ctx.crp_mode,
            depth,
            kind.as_deref(),
        );

        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some(action),
            path: None,
        })
    }
}
