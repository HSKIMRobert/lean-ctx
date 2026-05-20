use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxCallTool;

impl McpTool for CtxCallTool {
    fn name(&self) -> &'static str {
        "ctx_call"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_call",
            "Invoke any of the 50+ lean-ctx tools by name. Use for tools not in the core set.\n\
             CATEGORIES:\n\
             arch: ctx_architecture, ctx_impact, ctx_callgraph, ctx_refactor, ctx_symbol, ctx_routes, ctx_smells, ctx_index\n\
             debug: ctx_benchmark, ctx_verify, ctx_analyze, ctx_profile, ctx_proof, ctx_review\n\
             memory: ctx_semantic_search, ctx_artifacts\n\
             batch: ctx_fill, ctx_execute, ctx_expand, ctx_pack, ctx_plan, ctx_control, ctx_compile\n\
             agent: ctx_agent, ctx_share, ctx_task, ctx_handoff, ctx_workflow\n\
             util: ctx_compress, ctx_cache, ctx_retrieve, ctx_metrics, ctx_radar, ctx_dedup, ctx_cost, ctx_gain, ctx_heatmap, ctx_feedback, ctx_ledger, ctx_preload\n\
             Example: ctx_call({\"name\":\"ctx_architecture\",\"arguments\":{\"action\":\"overview\"}})",
            json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Tool name to invoke (e.g. 'ctx_architecture', 'ctx_benchmark')"
                    },
                    "arguments": {
                        "type": "object",
                        "description": "Arguments object to pass to the invoked tool"
                    }
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
