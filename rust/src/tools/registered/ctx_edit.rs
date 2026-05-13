use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_bool, get_int, get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxEditTool;

impl McpTool for CtxEditTool {
    fn name(&self) -> &'static str {
        "ctx_edit"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_edit",
            "Edit a file via search-and-replace. Works without native Read/Edit tools. Use this when the IDE's Edit tool requires Read but Read is unavailable.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path" },
                    "old_string": { "type": "string", "description": "Exact text to find and replace (must be unique unless replace_all=true)" },
                    "new_string": { "type": "string", "description": "Replacement text" },
                    "replace_all": { "type": "boolean", "description": "Replace all occurrences (default: false)", "default": false },
                    "create": { "type": "boolean", "description": "Create a new file with new_string as content (ignores old_string)", "default": false }
                },
                "required": ["path", "new_string"]
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let path = ctx
            .resolved_path("path")
            .ok_or_else(|| ErrorData::invalid_params("path is required", None))?
            .to_string();

        let old_string = get_str(args, "old_string").unwrap_or_default();
        let new_string = get_str(args, "new_string")
            .ok_or_else(|| ErrorData::invalid_params("new_string is required", None))?;
        let replace_all = get_bool(args, "replace_all").unwrap_or(false);
        let create = get_bool(args, "create").unwrap_or(false);
        let expected_md5 = get_str(args, "expected_md5");
        let expected_size = get_int(args, "expected_size").and_then(|v| u64::try_from(v).ok());
        let expected_mtime_ms =
            get_int(args, "expected_mtime_ms").and_then(|v| u64::try_from(v).ok());
        let backup = get_bool(args, "backup").unwrap_or(false);
        let backup_path = get_str(args, "backup_path")
            .map(|p| ctx.resolved_paths.get("backup_path").cloned().unwrap_or(p));
        let evidence = get_bool(args, "evidence").unwrap_or(true);
        let diff_max_lines = get_int(args, "diff_max_lines")
            .and_then(|v| usize::try_from(v.max(0)).ok())
            .unwrap_or(200);
        let allow_lossy_utf8 = get_bool(args, "allow_lossy_utf8").unwrap_or(false);

        tokio::task::block_in_place(|| {
            let cache_lock = ctx
                .cache
                .as_ref()
                .ok_or_else(|| ErrorData::internal_error("cache not available", None))?;
            let mut cache = cache_lock.blocking_write();
            let output = crate::tools::ctx_edit::handle(
                &mut cache,
                &crate::tools::ctx_edit::EditParams {
                    path: path.clone(),
                    old_string,
                    new_string,
                    replace_all,
                    create,
                    expected_md5,
                    expected_size,
                    expected_mtime_ms,
                    backup,
                    backup_path,
                    evidence,
                    diff_max_lines,
                    allow_lossy_utf8,
                },
            );
            drop(cache);

            if let Some(session_lock) = ctx.session.as_ref() {
                let mut session = session_lock.blocking_write();
                session.mark_modified(&path);
            }

            Ok(ToolOutput {
                text: output,
                original_tokens: 0,
                saved_tokens: 0,
                mode: None,
                path: Some(path),
            })
        })
    }
}
