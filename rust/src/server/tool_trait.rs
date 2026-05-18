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
    /// True when the tool mutated state that clients should know about
    /// (e.g. dynamic tool categories changed).
    pub changed: bool,
}

impl ToolOutput {
    pub fn simple(text: String) -> Self {
        Self {
            text,
            original_tokens: 0,
            saved_tokens: 0,
            mode: None,
            path: None,
            changed: false,
        }
    }

    /// Compact one-line summary for headers_only response verbosity.
    pub fn to_header_line(&self, tool_name: &str) -> String {
        let path_str = self.path.as_deref().unwrap_or("—");
        let mode_str = self.mode.as_deref().unwrap_or("—");
        let sent = self.original_tokens.saturating_sub(self.saved_tokens);
        let pct = if self.original_tokens > 0 {
            (self.saved_tokens as f64 / self.original_tokens as f64 * 100.0) as u32
        } else {
            0
        };
        format!("[{tool_name}: {path_str}, mode={mode_str}, {sent} tok sent, -{pct}%]")
    }

    pub fn with_savings(text: String, original: usize, saved: usize) -> Self {
        Self {
            text,
            original_tokens: original,
            saved_tokens: saved,
            mode: None,
            path: None,
            changed: false,
        }
    }
}

/// Trait for a self-contained MCP tool. Each tool provides its own schema
/// definition and handler, eliminating the possibility of schema/handler drift.
///
/// Handlers are synchronous because all existing tool handlers are sync.
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

/// Context passed to tool handlers. Contains pre-resolved values that
/// many tools need, avoiding repeated async lock acquisition inside
/// handlers. Extended with shared server state for tools that need
/// cache/session access.
pub struct ToolContext {
    pub project_root: String,
    pub minimal: bool,
    /// Pre-resolved paths keyed by argument name (e.g. "path" -> "/abs/dir").
    pub resolved_paths: std::collections::HashMap<String, String>,
    /// CRP mode for compression-aware tools.
    pub crp_mode: crate::tools::CrpMode,
    /// Shared cache handle for tools that need read/write access.
    pub cache: Option<crate::tools::SharedCache>,
    /// Shared session handle for tools that need session access.
    pub session: Option<std::sync::Arc<tokio::sync::RwLock<crate::core::session::SessionState>>>,
    /// Tool call records for session-aware tools (e.g. ctx_session status).
    pub tool_calls:
        Option<std::sync::Arc<tokio::sync::RwLock<Vec<crate::core::protocol::ToolCallRecord>>>>,
    /// Current agent identity for multi-agent tools.
    pub agent_id: Option<std::sync::Arc<tokio::sync::RwLock<Option<String>>>>,
    /// Active workflow run state.
    pub workflow:
        Option<std::sync::Arc<tokio::sync::RwLock<Option<crate::core::workflow::WorkflowRun>>>>,
    /// Context ledger for handoff operations.
    pub ledger:
        Option<std::sync::Arc<tokio::sync::RwLock<crate::core::context_ledger::ContextLedger>>>,
    /// Client name (cursor, claude, etc.).
    pub client_name: Option<std::sync::Arc<tokio::sync::RwLock<String>>>,
    /// Pipeline stats for metrics/proof tools.
    pub pipeline_stats:
        Option<std::sync::Arc<tokio::sync::RwLock<crate::core::pipeline::PipelineStats>>>,
    /// Global call counter for context tools.
    pub call_count: Option<std::sync::Arc<std::sync::atomic::AtomicUsize>>,
    /// Autonomy state for search repeat detection.
    pub autonomy: Option<std::sync::Arc<crate::tools::autonomy::AutonomyState>>,
    /// Pre-computed context pressure snapshot for synchronous gate decisions.
    pub pressure_snapshot: Option<crate::core::context_ledger::ContextPressure>,
    /// Errors from path resolution (PathJail rejection, secret path, etc.).
    /// Keyed by argument name (e.g. "path" -> "path escapes project root: ...").
    pub path_errors: std::collections::HashMap<String, String>,
}

impl ToolContext {
    pub fn resolved_path(&self, arg: &str) -> Option<&str> {
        self.resolved_paths.get(arg).map(String::as_str)
    }

    /// Returns the path resolution error for a given key, if any.
    pub fn path_error(&self, key: &str) -> Option<&str> {
        self.path_errors.get(key).map(String::as_str)
    }

    /// Sync path resolution using project_root. Simplified version
    /// of LeanCtxServer::resolve_path for use in sync tool handlers.
    pub fn resolve_path_sync(&self, path: &str) -> Result<String, String> {
        let normalized = crate::core::pathutil::normalize_tool_path(path);
        if normalized.is_empty() || normalized == "." {
            return Ok(normalized);
        }
        let p = std::path::Path::new(&normalized);
        let resolved = if p.is_absolute() || p.exists() {
            std::path::PathBuf::from(&normalized)
        } else {
            let joined = std::path::Path::new(&self.project_root).join(&normalized);
            if joined.exists() {
                joined
            } else {
                std::path::Path::new(&self.project_root).join(&normalized)
            }
        };
        let jail_root = std::path::Path::new(&self.project_root);
        let jailed = crate::core::pathjail::jail_path(&resolved, jail_root)?;
        crate::core::io_boundary::check_secret_path_for_tool("resolve_path", &jailed)?;
        Ok(crate::core::pathutil::normalize_tool_path(
            &jailed.to_string_lossy().replace('\\', "/"),
        ))
    }
}

// ── Arg extraction helpers (mirror server/helpers.rs for standalone use) ──

/// Extract a resolved path from context with differentiated error messages.
/// Returns descriptive errors for: missing param, PathJail rejection, wrong type.
pub fn require_resolved_path(
    ctx: &ToolContext,
    args: &Map<String, Value>,
    key: &str,
) -> Result<String, ErrorData> {
    if let Some(path) = ctx.resolved_path(key) {
        return Ok(path.to_string());
    }
    if let Some(err) = ctx.path_error(key) {
        return Err(ErrorData::invalid_params(format!("{key}: {err}"), None));
    }
    if let Some(val) = args.get(key) {
        if !val.is_string() {
            let type_name = match val {
                Value::Number(_) => "number",
                Value::Bool(_) => "boolean",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                Value::Null => "null",
                Value::String(_) => unreachable!(),
            };
            return Err(ErrorData::invalid_params(
                format!("{key} must be a string, got {type_name}"),
                None,
            ));
        }
    }
    Err(ErrorData::invalid_params(
        format!("{key} is required"),
        None,
    ))
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn empty_ctx() -> ToolContext {
        ToolContext {
            project_root: String::new(),
            minimal: false,
            resolved_paths: std::collections::HashMap::new(),
            crp_mode: crate::tools::CrpMode::Off,
            cache: None,
            session: None,
            tool_calls: None,
            agent_id: None,
            workflow: None,
            ledger: None,
            client_name: None,
            pipeline_stats: None,
            call_count: None,
            autonomy: None,
            pressure_snapshot: None,
            path_errors: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn require_resolved_path_returns_resolved() {
        let mut ctx = empty_ctx();
        ctx.resolved_paths
            .insert("path".to_string(), "/abs/file.rs".to_string());
        let args: Map<String, Value> = Map::new();
        let result = require_resolved_path(&ctx, &args, "path");
        assert_eq!(result.unwrap(), "/abs/file.rs");
    }

    #[test]
    fn require_resolved_path_surfaces_jail_error() {
        let mut ctx = empty_ctx();
        ctx.path_errors.insert(
            "path".to_string(),
            "path escapes project root /project".to_string(),
        );
        let args: Map<String, Value> = Map::new();
        let result = require_resolved_path(&ctx, &args, "path");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("escapes project root"), "got: {msg}");
    }

    #[test]
    fn require_resolved_path_detects_non_string() {
        let ctx = empty_ctx();
        let mut args: Map<String, Value> = Map::new();
        args.insert("path".to_string(), json!(42));
        let result = require_resolved_path(&ctx, &args, "path");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("must be a string, got number"), "got: {msg}");
    }

    #[test]
    fn require_resolved_path_missing_param() {
        let ctx = empty_ctx();
        let args: Map<String, Value> = Map::new();
        let result = require_resolved_path(&ctx, &args, "path");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("path is required"), "got: {msg}");
    }
}
