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
            "Retrieve firewalled/archived tool output (zero-loss). Large outputs are stored out-of-band and replaced inline by a digest+ref; use this to drill into the full content. Actions: retrieve (default), list, search_all.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Archive ID from the [Firewalled: ...]/[Archived: ...] hint (or a handle ref like @F1)" },
                    "action": { "type": "string", "description": "retrieve (default), list, or search_all" },
                    "start_line": { "type": "integer", "description": "Start line for range retrieval" },
                    "end_line": { "type": "integer", "description": "End line for range retrieval" },
                    "head": { "type": "integer", "description": "Return the first N lines" },
                    "tail": { "type": "integer", "description": "Return the last N lines" },
                    "search": { "type": "string", "description": "Return only lines matching this substring" },
                    "json_keys": { "type": "boolean", "description": "Describe the JSON structure (top-level keys, array lengths, type hints)" },
                    "json_path": { "type": "string", "description": "Navigate into JSON first (dot/slash path, e.g. data.items.0) before describing" },
                    "query": { "type": "string", "description": "Full-text query across all archives (action=search_all)" },
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
