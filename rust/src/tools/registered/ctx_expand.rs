use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxExpandTool;

impl McpTool for CtxExpandTool {
    fn name(&self) -> &'static str {
        "ctx_expand"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_expand",
            "Retrieve archived tool output (zero-loss). Large outputs are auto-archived; use this to retrieve full details. Actions: retrieve (default), list.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Archive ID from the [Archived: ...] hint" },
                    "action": { "type": "string", "description": "retrieve (default) or list" },
                    "start_line": { "type": "integer", "description": "Start line for range retrieval" },
                    "end_line": { "type": "integer", "description": "End line for range retrieval" },
                    "search": { "type": "string", "description": "Search pattern to filter archived output" },
                    "session_id": { "type": "string", "description": "Filter list by session ID" }
                }
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        _ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let args_val = Value::Object(args.clone());
        let result = crate::tools::ctx_expand::handle(&args_val);
        Ok(ToolOutput::simple(result))
    }
}
