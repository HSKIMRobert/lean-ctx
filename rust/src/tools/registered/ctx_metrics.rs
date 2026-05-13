use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxMetricsTool;

impl McpTool for CtxMetricsTool {
    fn name(&self) -> &'static str {
        "ctx_metrics"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_metrics",
            "Session token stats, cache rates, per-tool savings.",
            json!({
                "type": "object",
                "properties": {}
            }),
        )
    }

    fn handle(
        &self,
        _args: &Map<String, Value>,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let cache = ctx.cache.as_ref().unwrap();
        let cache_guard = tokio::task::block_in_place(|| cache.blocking_read());
        let calls = ctx.tool_calls.as_ref().unwrap();
        let calls_guard = tokio::task::block_in_place(|| calls.blocking_read());
        let mut result =
            crate::tools::ctx_metrics::handle(&cache_guard, &calls_guard, ctx.crp_mode);
        drop(cache_guard);
        drop(calls_guard);

        if let Some(ref ps) = ctx.pipeline_stats {
            let stats = tokio::task::block_in_place(|| ps.blocking_read());
            if stats.runs > 0 {
                result.push_str("\n\n--- PIPELINE METRICS ---\n");
                result.push_str(&stats.format_summary());
            }
        }

        let (ts_hits, regex_hits) = crate::core::signatures::signature_backend_stats();
        if ts_hits + regex_hits > 0 {
            result.push_str("\n--- SIGNATURE BACKEND ---\n");
            result.push_str(&format!(
                "tree-sitter: {} | regex fallback: {} | ratio: {:.0}%\n",
                ts_hits,
                regex_hits,
                if ts_hits + regex_hits > 0 {
                    ts_hits as f64 / (ts_hits + regex_hits) as f64 * 100.0
                } else {
                    0.0
                }
            ));
        }

        Ok(ToolOutput::simple(result))
    }
}
