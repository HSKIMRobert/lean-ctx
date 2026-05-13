use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_bool, get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxSessionTool;

impl McpTool for CtxSessionTool {
    fn name(&self) -> &'static str {
        "ctx_session"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_session",
            "Cross-session memory (CCP). Actions: load (restore ~400 tok), save, status, task, \
finding, decision, reset, list, cleanup, snapshot, restore, resume, profile (context profiles), \
role (governance), budget (limits), slo (observability), diff (compare sessions), verify (output verification stats), \
episodes (episodic memory), procedures (procedural memory).",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "load", "save", "task", "finding", "decision", "reset", "list", "cleanup", "snapshot", "restore", "resume", "profile", "role", "budget", "slo", "diff", "verify", "episodes", "procedures"],
                        "description": "Session operation to perform"
                    },
                    "value": {
                        "type": "string",
                        "description": "Value for task/finding/decision/profile actions"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Session ID for load action (default: latest)"
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
        let value = get_str(args, "value");
        let sid = get_str(args, "session_id");
        let format = get_str(args, "format");
        let path = get_str(args, "path");
        let write = get_bool(args, "write").unwrap_or(false);
        let privacy = get_str(args, "privacy");
        let terse = get_bool(args, "terse");

        let tool_calls_handle = ctx.tool_calls.as_ref().unwrap();
        let call_durations: Vec<(String, u64)> = {
            let tc = tool_calls_handle.blocking_read();
            tc.iter().map(|c| (c.tool.clone(), c.duration_ms)).collect()
        };

        let session_handle = ctx.session.as_ref().unwrap();
        let mut session = session_handle.blocking_write();
        let result = crate::tools::ctx_session::handle(
            &mut session,
            &call_durations,
            &action,
            value.as_deref(),
            sid.as_deref(),
            crate::tools::ctx_session::SessionToolOptions {
                format: format.as_deref(),
                path: path.as_deref(),
                write,
                privacy: privacy.as_deref(),
                terse,
            },
        );
        drop(session);

        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some(action),
            path: None,
        })
    }
}
