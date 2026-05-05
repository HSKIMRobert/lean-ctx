use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxBenchmarkTool;

impl McpTool for CtxBenchmarkTool {
    fn name(&self) -> &'static str {
        "ctx_benchmark"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_benchmark",
            "Benchmark compression modes for a file or project.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "action": { "type": "string" },
                    "format": { "type": "string" }
                },
                "required": ["path"]
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

        let action = get_str(args, "action").unwrap_or_default();
        let result = if action == "project" {
            let fmt = get_str(args, "format").unwrap_or_default();
            let bench = crate::core::benchmark::run_project_benchmark(&path);
            match fmt.as_str() {
                "json" => crate::core::benchmark::format_json(&bench),
                "markdown" | "md" => crate::core::benchmark::format_markdown(&bench),
                _ => crate::core::benchmark::format_terminal(&bench),
            }
        } else {
            crate::tools::ctx_benchmark::handle(&path, crate::tools::CrpMode::effective())
        };

        Ok(ToolOutput::simple(result))
    }
}
