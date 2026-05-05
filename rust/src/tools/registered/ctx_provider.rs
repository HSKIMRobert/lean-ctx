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
            "External context provider (GitLab-first). Actions: gitlab_issues (list), gitlab_issue (show by iid), gitlab_mrs (list MRs), gitlab_pipelines (list pipelines). \
             Requires GITLAB_TOKEN or LEAN_CTX_GITLAB_TOKEN.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["gitlab_issues", "gitlab_issue", "gitlab_mrs", "gitlab_pipelines"],
                        "description": "Provider action"
                    },
                    "state": {
                        "type": "string",
                        "description": "Filter by state (opened, closed, merged, all)"
                    },
                    "labels": {
                        "type": "string",
                        "description": "Comma-separated labels filter"
                    },
                    "iid": {
                        "type": "integer",
                        "description": "Issue/MR IID for single-item lookup"
                    },
                    "status": {
                        "type": "string",
                        "description": "Pipeline status filter (running, success, failed)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default 20, max 100)"
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
        let result = crate::tools::ctx_provider::handle(args);
        Ok(ToolOutput::simple(result))
    }
}
