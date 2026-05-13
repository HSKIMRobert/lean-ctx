use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxSmartReadTool;

impl McpTool for CtxSmartReadTool {
    fn name(&self) -> &'static str {
        "ctx_smart_read"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_smart_read",
            "Auto-select optimal read mode for a file.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path to read" }
                },
                "required": ["path"]
            }),
        )
    }

    fn handle(
        &self,
        _args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let path = ctx
            .resolved_path("path")
            .ok_or_else(|| ErrorData::invalid_params("path is required", None))?
            .to_string();

        if crate::core::binary_detect::is_binary_file(&path) {
            let msg = crate::core::binary_detect::binary_file_message(&path);
            return Err(ErrorData::invalid_params(msg, None));
        }
        {
            let cap = crate::core::limits::max_read_bytes() as u64;
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.len() > cap {
                    let msg = format!(
                        "File too large ({} bytes, limit {} bytes via LCTX_MAX_READ_BYTES). \
                         Use mode=\"lines:1-100\" for partial reads or increase the limit.",
                        meta.len(),
                        cap
                    );
                    return Err(ErrorData::invalid_params(msg, None));
                }
            }
        }

        tokio::task::block_in_place(|| {
            let cache_lock = ctx
                .cache
                .as_ref()
                .ok_or_else(|| ErrorData::internal_error("cache not available", None))?;
            let mut cache = cache_lock.blocking_write();
            let output = crate::tools::ctx_smart_read::handle(&mut cache, &path, ctx.crp_mode);
            let original = cache.get(&path).map_or(0, |e| e.original_tokens);
            let tokens = crate::core::tokens::count_tokens(&output);
            drop(cache);

            let saved = original.saturating_sub(tokens);
            Ok(ToolOutput {
                text: output,
                original_tokens: original,
                saved_tokens: saved,
                mode: Some("auto".to_string()),
                path: Some(path),
            })
        })
    }
}
