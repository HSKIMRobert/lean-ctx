use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxOverviewTool;

impl McpTool for CtxOverviewTool {
    fn name(&self) -> &'static str {
        "ctx_overview"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_overview",
            "Task-relevant project map — use at session start.",
            json!({
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Task description for relevance scoring"
                    },
                    "path": {
                        "type": "string",
                        "description": "Project root directory (default: .)"
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
        let task = get_str(args, "task");

        let resolved_path = if get_str(args, "path").is_some() {
            ctx.resolved_path("path").map(String::from)
        } else if let Some(ref session) = ctx.session {
            let guard = tokio::task::block_in_place(|| session.blocking_read());
            guard.project_root.clone()
        } else {
            None
        };

        let cache = ctx.cache.as_ref().unwrap();
        let guard = tokio::task::block_in_place(|| cache.blocking_read());
        let result = crate::tools::ctx_overview::handle(
            &guard,
            task.as_deref(),
            resolved_path.as_deref(),
            ctx.crp_mode,
        );

        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some("overview".to_string()),
            path: None,
        })
    }
}
