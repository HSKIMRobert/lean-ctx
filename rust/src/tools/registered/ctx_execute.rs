use rmcp::ErrorData;
use rmcp::model::Tool;
use serde_json::{Map, Value, json};

use crate::server::tool_trait::{
    McpTool, ToolContext, ToolOutput, get_int, get_str, require_resolved_path,
};
use crate::tool_defs::tool_def;

pub struct CtxExecuteTool;

impl McpTool for CtxExecuteTool {
    fn name(&self) -> &'static str {
        "ctx_execute"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_execute",
            "Run code in sandbox (11 languages) — use when compute beats shell glue.\n\
             action=code (default) for one-shot transform/math/generation; action=batch for parallel\n\
             multi-language scripts; action=file to process a project file (extension auto-detects\n\
             language). Pass intent to focus large output and save tokens. Prefer over ctx_shell when\n\
             logic, conditionals, multi-line scripts, or cross-language data munging — stdout-only,\n\
             no argv escaping. Languages: javascript, typescript, python, shell, ruby, go, rust, php,\n\
             perl, r, elixir.",
            json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "description": "REQUIRED for action=code. One of: javascript|typescript|python|shell|ruby|go|rust|php|perl|r|elixir. Omit for action=file (auto-detected)."
                    },
                    "code": {
                        "type": "string",
                        "description": "Source code string. REQUIRED for action=code. Set intent to focus large output on what you need."
                    },
                    "intent": {
                        "type": "string",
                        "description": "What you need from output. Triggers intent-driven filtering when output is large — saves tokens by discarding irrelevant lines."
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 30). Increase for long-running scripts (data processing, scraping)."
                    },
                    "action": {
                        "type": "string",
                        "description": "code (default) — run one script via language+code. batch — run multiple scripts in parallel via items. file — process a project file (language auto-detected from extension)."
                    },
                    "items": {
                        "type": "string",
                        "description": "JSON array of [{\"language\": \"...\", \"code\": \"...\"}] for action=batch. All scripts run in parallel."
                    },
                    "path": {
                        "type": "string",
                        "description": "Project file path. REQUIRED for action=file. Language auto-detected from file extension (see detect_language_from_extension)."
                    }
                }
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let action = get_str(args, "action").unwrap_or_default();

        let (result, outcome) = if action == "batch" {
            let items_str = get_str(args, "items")
                .ok_or_else(|| ErrorData::invalid_params("items is required for batch", None))?;
            let items: Vec<serde_json::Value> = serde_json::from_str(&items_str)
                .map_err(|e| ErrorData::invalid_params(format!("Invalid items JSON: {e}"), None))?;
            let batch: Vec<(String, String)> = items
                .iter()
                .filter_map(|item| {
                    let lang = item.get("language")?.as_str()?.to_string();
                    let code = item.get("code")?.as_str()?.to_string();
                    Some((lang, code))
                })
                .collect();
            crate::tools::ctx_execute::handle_batch(&batch)
        } else if action == "file" {
            let path = require_resolved_path(ctx, args, "path")?;
            let project_root = if ctx.project_root.is_empty() {
                None
            } else {
                Some(ctx.project_root.as_str())
            };
            let intent = get_str(args, "intent");
            crate::tools::ctx_execute::handle_file(&path, intent.as_deref(), project_root)
        } else {
            let language = get_str(args, "language")
                .ok_or_else(|| ErrorData::invalid_params("language is required", None))?;
            let code = get_str(args, "code")
                .ok_or_else(|| ErrorData::invalid_params("code is required", None))?;
            let intent = get_str(args, "intent");
            let timeout = get_int(args, "timeout").and_then(|t| u64::try_from(t).ok());
            crate::tools::ctx_execute::handle(&language, &code, intent.as_deref(), timeout)
        };

        let result = crate::core::redaction::redact_text_if_enabled(&result);
        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some(action),
            path: None,
            changed: false,
            shell_outcome: Some(outcome),
        })
    }
}
