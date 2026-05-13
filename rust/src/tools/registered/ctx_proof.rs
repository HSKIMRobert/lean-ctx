use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_bool, get_int, get_str, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxProofTool;

impl McpTool for CtxProofTool {
    fn name(&self) -> &'static str {
        "ctx_proof"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_proof",
            "Export a machine-readable ContextProofV1 (Verifier + SLO + Pipeline + Provenance). Writes to .lean-ctx/proofs/ by default.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "description": "export (required)" },
                    "project_root": { "type": "string", "description": "Project root for proof output (default: .)" },
                    "format": { "type": "string", "description": "json|summary|both (default: json)" },
                    "write": { "type": "boolean", "description": "Write proof file under .lean-ctx/proofs/ (default: true)" },
                    "filename": { "type": "string", "description": "Optional output filename" },
                    "max_evidence": { "type": "integer", "description": "Max tool receipts to include (default: 50)" },
                    "max_ledger_files": { "type": "integer", "description": "Max context ledger top files to include (default: 10)" }
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
        if action != "export" {
            return Err(ErrorData::invalid_params(
                "unsupported action (expected: export)",
                None,
            ));
        }

        let root = ctx
            .resolved_path("project_root")
            .unwrap_or(&ctx.project_root)
            .to_string();
        let format = get_str(args, "format");
        let write = get_bool(args, "write").unwrap_or(true);
        let filename = get_str(args, "filename");
        let max_evidence = get_int(args, "max_evidence").map(|v| v as usize);
        let max_ledger_files = get_int(args, "max_ledger_files").map(|v| v as usize);

        let session_data = ctx
            .session
            .as_ref()
            .map(|s| tokio::task::block_in_place(|| s.blocking_read()).clone());
        let pipeline_data = ctx
            .pipeline_stats
            .as_ref()
            .map(|p| tokio::task::block_in_place(|| p.blocking_read()).clone());
        let ledger_data = ctx
            .ledger
            .as_ref()
            .map(|l| tokio::task::block_in_place(|| l.blocking_read()).clone());

        let sources = crate::core::context_proof::ProofSources {
            project_root: Some(root.clone()),
            session: session_data,
            pipeline: pipeline_data,
            ledger: ledger_data,
        };

        let out = crate::tools::ctx_proof::handle_export(
            &root,
            format.as_deref(),
            write,
            filename.as_deref(),
            max_evidence,
            max_ledger_files,
            sources,
        )
        .map_err(|e| ErrorData::invalid_params(e, None))?;

        Ok(ToolOutput {
            text: out,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some(action),
            path: Some(root),
        })
    }
}
