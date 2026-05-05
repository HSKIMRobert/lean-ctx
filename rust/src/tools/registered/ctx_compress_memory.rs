use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxCompressMemoryTool;

impl McpTool for CtxCompressMemoryTool {
    fn name(&self) -> &'static str {
        "ctx_compress_memory"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_compress_memory",
            "Compress a memory/config file (CLAUDE.md, .cursorrules) preserving code, URLs, paths. Creates .original.md backup.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
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

        let result = crate::tools::ctx_compress_memory::handle(&path);
        Ok(ToolOutput::simple(result))
    }
}
