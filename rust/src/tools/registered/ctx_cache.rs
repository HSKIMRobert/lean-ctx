use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxCacheTool;

impl McpTool for CtxCacheTool {
    fn name(&self) -> &'static str {
        "ctx_cache"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_cache",
            "Cache ops: status|clear|invalidate.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "clear", "invalidate"],
                        "description": "Cache operation to perform"
                    },
                    "path": {
                        "type": "string",
                        "description": "File path (required for 'invalidate' action)"
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

        let invalidate_path = if action == "invalidate" {
            Some(
                ctx.resolved_path("path")
                    .ok_or_else(|| {
                        ErrorData::invalid_params("path is required for invalidate", None)
                    })?
                    .to_string(),
            )
        } else {
            None
        };

        let cache = ctx.cache.as_ref().unwrap();
        let mut guard = tokio::task::block_in_place(|| cache.blocking_write());

        let result = match action.as_str() {
            "status" => {
                let entries = guard.get_all_entries();
                if entries.is_empty() {
                    "Cache empty — no files tracked.".to_string()
                } else {
                    let mut lines = vec![format!("Cache: {} file(s)", entries.len())];
                    for (path, entry) in &entries {
                        let fref = guard
                            .file_ref_map()
                            .get(*path)
                            .map_or("F?", std::string::String::as_str);
                        lines.push(format!(
                            "  {fref}={} [{}L, {}t, read {}x]",
                            crate::core::protocol::shorten_path(path),
                            entry.line_count,
                            entry.original_tokens,
                            entry.read_count
                        ));
                    }
                    lines.join("\n")
                }
            }
            "clear" => {
                let count = guard.clear();
                format!(
                    "Cache cleared — {count} file(s) removed. Next ctx_read will return full content."
                )
            }
            "invalidate" => {
                let Some(path) = invalidate_path else {
                    return Ok(ToolOutput::simple(
                        "Missing path for invalidate action.".to_string(),
                    ));
                };
                if guard.invalidate(&path) {
                    format!(
                        "Invalidated cache for {}. Next ctx_read will return full content.",
                        crate::core::protocol::shorten_path(&path)
                    )
                } else {
                    format!(
                        "{} was not in cache.",
                        crate::core::protocol::shorten_path(&path)
                    )
                }
            }
            _ => "Unknown action. Use: status, clear, invalidate".to_string(),
        };

        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some(action),
            path: None,
        })
    }
}
