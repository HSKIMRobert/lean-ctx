use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{get_str, get_usize, McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxGraphTool;

impl McpTool for CtxGraphTool {
    fn name(&self) -> &'static str {
        "ctx_graph"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_graph",
            "Unified code graph. Actions: build (index), related (connected files), symbol (def/usages), \
impact (blast radius), status (stats), enrich (add commits+tests+knowledge), context (task-based query), diagram (Mermaid deps/calls), \
neighbors (direct in/out edges of a file), path (shortest connection between two files), explain (why a file matters: degree/community/bridge), diff (files changed since a git ref + their blast radius).",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["build", "related", "symbol", "impact", "status", "enrich", "context", "diagram", "neighbors", "path", "explain", "diff"],
                        "description": "Graph operation"
                    },
                    "path": {
                        "type": "string",
                        "description": "File path (related/impact/neighbors/explain), file::symbol_name (symbol), or the FROM file (path)"
                    },
                    "to": {
                        "type": "string",
                        "description": "Target file for action=path (shortest path destination)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Optional traversal depth for action=diagram (default 2) and action=neighbors (default 1)"
                    },
                    "kind": {
                        "type": "string",
                        "description": "Optional kind for action=diagram: deps|calls"
                    },
                    "format": {
                        "type": "string",
                        "description": "Output format for neighbors/path/explain/diff: text (default) or json"
                    },
                    "since": {
                        "type": "string",
                        "description": "Base git ref for action=diff (default HEAD~1), e.g. a commit SHA, tag or HEAD~5"
                    },
                    "project_root": {
                        "type": "string",
                        "description": "Project root directory (default: .)"
                    }
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

        // For diagram action, pass the raw path; for others, use the resolved path.
        let path = if action == "diagram" {
            get_str(args, "path")
        } else if let Some(p) = ctx.resolved_path("path") {
            Some(p.to_string())
        } else if ctx.path_error("path").is_some() && get_str(args, "path").is_some() {
            return Err(ErrorData::invalid_params(
                format!("path: {}", ctx.path_error("path").unwrap()),
                None,
            ));
        } else {
            None
        };

        let root = if let Some(p) = ctx.resolved_path("project_root") {
            p.to_string()
        } else if let Some(err) = ctx.path_error("project_root") {
            return Err(ErrorData::invalid_params(
                format!("project_root: {err}"),
                None,
            ));
        } else {
            ctx.project_root.clone()
        };
        let depth = get_usize(args, "depth").map(|d| d.min(64));
        let kind = get_str(args, "kind");
        let format = get_str(args, "format");
        // `since` is a git ref, not a filesystem path — read it raw (no PathJail).
        let since = get_str(args, "since");
        let to = if let Some(p) = ctx.resolved_path("to") {
            Some(p.to_string())
        } else if ctx.path_error("to").is_some() && get_str(args, "to").is_some() {
            return Err(ErrorData::invalid_params(
                format!("to: {}", ctx.path_error("to").unwrap()),
                None,
            ));
        } else {
            None
        };

        let cache = ctx
            .cache
            .as_ref()
            .ok_or_else(|| ErrorData::internal_error("cache not available", None))?;
        let Some(mut guard) = crate::server::bounded_lock::write(cache, "ctx_graph") else {
            return Ok(ToolOutput::simple(
                "[graph cache temporarily unavailable — retry in a moment]".to_string(),
            ));
        };
        let result = crate::tools::ctx_graph::handle(
            &action,
            path.as_deref(),
            &root,
            &mut guard,
            ctx.crp_mode,
            depth,
            kind.as_deref(),
            to.as_deref(),
            format.as_deref(),
            since.as_deref(),
        );

        Ok(ToolOutput {
            text: result,
            original_tokens: 0,
            saved_tokens: 0,
            mode: Some(action),
            path: None,
            changed: false,
        })
    }
}
