use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxPreloadTool;

impl McpTool for CtxPreloadTool {
    fn name(&self) -> &'static str {
        "ctx_preload"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_preload",
            "Proactive context loader — caches task-relevant files, returns L-curve-optimized summary (~50-100 tokens vs ~5000 for individual reads).",
            json!({
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Task description (e.g. 'fix auth bug in validate_token')"
                    },
                    "path": {
                        "type": "string",
                        "description": "Project root (default: .)"
                    }
                },
                "required": ["task"]
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let task = get_str(args, "task").unwrap_or_default();

        let resolved_path = if get_str(args, "path").is_some() {
            ctx.resolved_path("path").map(String::from)
        } else if let Some(ref session) = ctx.session {
            let guard = tokio::task::block_in_place(|| session.blocking_read());
            guard.project_root.clone()
        } else {
            None
        };

        let cache = ctx.cache.as_ref().unwrap();
        let mut cache_guard = tokio::task::block_in_place(|| cache.blocking_write());
        let mut result = crate::tools::ctx_preload::handle(
            &mut cache_guard,
            &task,
            resolved_path.as_deref(),
            ctx.crp_mode,
        );
        drop(cache_guard);

        if let Some(ref session_lock) = ctx.session {
            let mut session_guard = tokio::task::block_in_place(|| session_lock.blocking_write());
            if session_guard.active_structured_intent.is_none()
                || session_guard
                    .active_structured_intent
                    .as_ref()
                    .is_none_or(|i| i.confidence < 0.6)
            {
                session_guard.set_task(&task, Some("preload"));
            }
            drop(session_guard);

            let session_guard = tokio::task::block_in_place(|| session_lock.blocking_read());
            if let Some(ref intent) = session_guard.active_structured_intent {
                if let Some(ref ledger_lock) = ctx.ledger {
                    let ledger = tokio::task::block_in_place(|| ledger_lock.blocking_read());
                    if !ledger.entries.is_empty() {
                        let known: Vec<String> = session_guard
                            .files_touched
                            .iter()
                            .map(|f| f.path.clone())
                            .collect();
                        let deficit =
                            crate::core::context_deficit::detect_deficit(&ledger, intent, &known);
                        if !deficit.suggested_files.is_empty() {
                            result.push_str("\n\n--- SUGGESTED FILES ---");
                            for s in &deficit.suggested_files {
                                result.push_str(&format!(
                                    "\n  {} ({:?}, ~{} tok, mode: {})",
                                    s.path, s.reason, s.estimated_tokens, s.recommended_mode
                                ));
                            }
                        }

                        let pressure = ledger.pressure();
                        if pressure.utilization > 0.7 {
                            let plan = ledger.reinjection_plan(intent, 0.6);
                            if !plan.actions.is_empty() {
                                result.push_str("\n\n--- REINJECTION PLAN ---");
                                result.push_str(&format!(
                                    "\n  Context pressure: {:.0}% -> target: 60%",
                                    pressure.utilization * 100.0
                                ));
                                for a in &plan.actions {
                                    result.push_str(&format!(
                                        "\n  {} : {} -> {} (frees ~{} tokens)",
                                        a.path, a.current_mode, a.new_mode, a.tokens_freed
                                    ));
                                }
                                result.push_str(&format!(
                                    "\n  Total freeable: {} tokens",
                                    plan.total_tokens_freed
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some("preload".to_string()),
            path: None,
        })
    }
}
