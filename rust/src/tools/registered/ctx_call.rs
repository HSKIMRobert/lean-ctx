use rmcp::ErrorData;
use rmcp::model::Tool;
use serde_json::{Map, Value, json};

use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput, get_str};
use crate::tool_defs::tool_def;

pub struct CtxCallTool;

impl McpTool for CtxCallTool {
    fn name(&self) -> &'static str {
        "ctx_call"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_call",
            "Invoke any non-core lean-ctx tool by name.\n\
             Categories: arch, debug, memory, batch, agent, util. Find exact names \
             with ctx_discover_tools (query=keyword; empty query lists all).",
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Tool name" },
                    "arguments": { "type": "object", "description": "Arguments for the tool" }
                },
                "required": ["name"]
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        _ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let name = get_str(args, "name")
            .ok_or_else(|| ErrorData::invalid_params("'name' is required", None))?;

        if name == "ctx_call" {
            return Err(ErrorData::invalid_params(
                "ctx_call cannot invoke itself",
                None,
            ));
        }

        Err(ErrorData::internal_error(
            format!(
                "ctx_call dispatch for '{name}' must be handled by the async dispatch layer. \
                 If you see this error, the tool was routed to the sync handler by mistake."
            ),
            None,
        ))
    }
}
