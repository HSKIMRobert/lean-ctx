use rmcp::model::Tool;
use rmcp::ErrorData;
use serde_json::{Map, Value};

/// Result returned by an McpTool handler.
pub struct ToolOutput {
    pub text: String,
    pub original_tokens: usize,
    pub saved_tokens: usize,
    pub mode: Option<String>,
    /// Path associated with the tool call (for record_call_with_path).
    pub path: Option<String>,
}

impl ToolOutput {
    pub fn simple(text: String) -> Self {
        Self {
            text,
            original_tokens: 0,
            saved_tokens: 0,
            mode: None,
            path: None,
        }
    }

    pub fn with_savings(text: String, original: usize, saved: usize) -> Self {
        Self {
            text,
            original_tokens: original,
            saved_tokens: saved,
            mode: None,
            path: None,
        }
    }
}

/// Trait for a self-contained MCP tool. Each tool provides its own schema
/// definition and handler, eliminating the possibility of schema/handler drift.
///
/// Handlers are synchronous because all 47 existing tool handlers are sync.
/// The async boundary (cache locks, session reads) is handled by the dispatch
/// layer before calling `handle`.
pub trait McpTool: Send + Sync {
    /// Tool name as registered in the MCP protocol (e.g. "ctx_tree").
    fn name(&self) -> &'static str;

    /// MCP tool definition including JSON schema. This replaces the
    /// corresponding entry in `granular_tool_defs()`.
    fn tool_def(&self) -> Tool;

    /// Execute the tool. Args are the raw JSON-RPC arguments.
    /// `ctx` provides access to resolved paths and project state.
    fn handle(&self, args: &Map<String, Value>, ctx: &ToolContext)
        -> Result<ToolOutput, ErrorData>;
}

/// Read-only context passed to tool handlers. Contains pre-resolved
/// values that many tools need, avoiding repeated async lock acquisition
/// inside handlers.
pub struct ToolContext {
    pub project_root: String,
    pub minimal: bool,
    /// Pre-resolved paths keyed by argument name (e.g. "path" -> "/abs/dir").
    /// Set by the dispatch layer before calling the handler so tools don't
    /// need async access to the session/pathJail.
    pub resolved_paths: std::collections::HashMap<String, String>,
}

impl ToolContext {
    pub fn resolved_path(&self, arg: &str) -> Option<&str> {
        self.resolved_paths.get(arg).map(String::as_str)
    }
}

// ── Arg extraction helpers (mirror server/helpers.rs for standalone use) ──

pub fn get_str(args: &Map<String, Value>, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(String::from)
}

pub fn get_int(args: &Map<String, Value>, key: &str) -> Option<i64> {
    args.get(key).and_then(serde_json::Value::as_i64)
}

pub fn get_bool(args: &Map<String, Value>, key: &str) -> Option<bool> {
    args.get(key).and_then(serde_json::Value::as_bool)
}

pub fn get_str_array(args: &Map<String, Value>, key: &str) -> Option<Vec<String>> {
    args.get(key).and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    })
}
