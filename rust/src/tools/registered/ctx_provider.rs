use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{json, Map, Value};

use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

pub struct CtxProviderTool;

impl McpTool for CtxProviderTool {
    fn name(&self) -> &'static str {
        "ctx_provider"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_provider",
            "External context providers (GitHub, GitLab, more). \
             Use action=discover to list available providers. \
             Use action=query with provider+resource for registry-based access. \
             Legacy GitLab actions still supported. \
             Set GITHUB_TOKEN or GITLAB_TOKEN to enable providers.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "discover",
                            "query",
                            "gitlab_issues",
                            "gitlab_issue",
                            "gitlab_mrs",
                            "gitlab_pipelines"
                        ],
                        "description": "Provider action. 'discover' lists all providers. 'query' uses registry routing (requires provider+resource)."
                    },
                    "provider": {
                        "type": "string",
                        "description": "Provider ID for action=query (e.g. 'github', 'gitlab')"
                    },
                    "resource": {
                        "type": "string",
                        "description": "Resource type for action=query (e.g. 'issues', 'pull_requests', 'actions')"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["compact", "chunks"],
                        "description": "Output mode for action=query. 'compact' (default) returns formatted text. 'chunks' returns ContentChunk metadata for BM25/embedding ingest."
                    },
                    "state": {
                        "type": "string",
                        "description": "Filter by state (open, closed, merged, all)"
                    },
                    "labels": {
                        "type": "string",
                        "description": "Comma-separated labels filter (GitLab)"
                    },
                    "iid": {
                        "type": "integer",
                        "description": "Issue/MR IID for single-item lookup (GitLab)"
                    },
                    "status": {
                        "type": "string",
                        "description": "Pipeline/Actions status filter (running, success, failed)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default 20, max 100)"
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
        let result = crate::tools::ctx_provider::handle(args, ctx);
        Ok(ToolOutput::simple(result))
    }
}
