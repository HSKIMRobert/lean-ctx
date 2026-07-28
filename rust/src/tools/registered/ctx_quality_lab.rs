//! ctx_quality_lab — run Quality Lab compression analysis via MCP.

use rmcp::model::{ErrorData, Tool};
use serde_json::{Map, Value, json};

use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub(crate) struct CtxQualityLabTool;

impl McpTool for CtxQualityLabTool {
    fn name(&self) -> &'static str {
        "ctx_quality_lab"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_quality_lab",
            "Run Quality Lab: compression fidelity, cache effectiveness, tokenizer calibration, ETPAO. Pass original+compressed text, or omit for runtime-only metrics.",
            json!({
                "type": "object",
                "properties": {
                    "original": {
                        "type": "string",
                        "description": "Original uncompressed text"
                    },
                    "compressed": {
                        "type": "string",
                        "description": "Compressed text to evaluate"
                    },
                    "ext": {
                        "type": "string",
                        "description": "File extension for fidelity analysis (default: rs)",
                        "default": "rs"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["json", "text"],
                        "description": "Output format (default: text)",
                        "default": "text"
                    }
                }
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        _ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let original = args.get("original").and_then(Value::as_str).unwrap_or("");
        let compressed = args.get("compressed").and_then(Value::as_str).unwrap_or("");
        let ext = args.get("ext").and_then(Value::as_str).unwrap_or("rs");
        let format = args.get("format").and_then(Value::as_str).unwrap_or("text");

        let report = crate::core::quality_lab::run_quality_lab(original, compressed, ext);

        let output = if format == "json" {
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        } else {
            crate::core::quality_lab::format_quality_report(&report)
        };

        Ok(ToolOutput::simple(output))
    }
}
