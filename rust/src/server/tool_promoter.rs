//! Session-local promotion of hidden tools based on observed tool usage.

use serde_json::{Map, Value};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

const CALLGRAPH_TOOLS: &[&str] = &["ctx_architecture", "ctx_impact", "ctx_graph"];
const SYMBOL_TOOLS: &[&str] = &["ctx_symbol", "ctx_refactor"];
const COMPOSE_TOOLS: &[&str] = &["ctx_overview", "ctx_repomap"];
const VERIFY_TOOLS: &[&str] = &["ctx_verify", "ctx_review"];
const SESSION_TOOLS: &[&str] = &["ctx_session", "ctx_knowledge"];

/// Tracks session tool usage to promote related hidden tools.
#[derive(Debug, Default)]
pub struct ToolPromoter {
    /// Tools explicitly promoted for this session.
    promoted: HashSet<String>,
    /// Number of tool calls since session start.
    call_count: u32,
}

impl ToolPromoter {
    fn record_call(&mut self, tool_name: &str, args: Option<&Map<String, Value>>) -> bool {
        self.call_count = self.call_count.saturating_add(1);
        let mut candidates = Vec::new();

        match tool_name {
            "ctx_callgraph" => candidates.extend_from_slice(CALLGRAPH_TOOLS),
            "ctx_search" if has_action(args, "symbol") => {
                candidates.extend_from_slice(SYMBOL_TOOLS);
            }
            "ctx_compose" => candidates.extend_from_slice(COMPOSE_TOOLS),
            "ctx_shell" if is_build_or_test_call(args) => {
                candidates.extend_from_slice(VERIFY_TOOLS);
            }
            _ => {}
        }
        if self.call_count >= 5 {
            candidates.extend_from_slice(SESSION_TOOLS);
        }

        candidates.into_iter().fold(false, |changed, name| {
            self.promoted.insert(name.to_string()) || changed
        })
    }
}

fn has_action(args: Option<&Map<String, Value>>, expected: &str) -> bool {
    args.and_then(|map| map.get("action"))
        .and_then(Value::as_str)
        .is_some_and(|action| action.eq_ignore_ascii_case(expected))
}

fn is_build_or_test_call(args: Option<&Map<String, Value>>) -> bool {
    let Some(command) = args
        .and_then(|map| map.get("command"))
        .and_then(Value::as_str)
    else {
        return false;
    };
    command.split_whitespace().any(|part| {
        let token = part
            .trim_matches(|character: char| !character.is_ascii_alphanumeric())
            .to_ascii_lowercase();
        matches!(
            token.as_str(),
            "build" | "test" | "tests" | "check" | "clippy" | "verify"
        )
    })
}

/// Record a tool call, returning whether it caused any new promotion.
pub fn record_call(tool_name: &str, args: Option<&Map<String, Value>>) -> bool {
    global()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .record_call(tool_name, args)
}

/// Return the names of all currently promoted tools in deterministic order.
#[must_use]
pub fn promoted_tools() -> Vec<String> {
    let mut tools: Vec<_> = global()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .promoted
        .iter()
        .cloned()
        .collect();
    tools.sort_unstable();
    tools
}

/// Check whether a tool has been promoted in the current session.
#[must_use]
pub fn is_promoted(tool_name: &str) -> bool {
    global()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .promoted
        .contains(tool_name)
}

/// Clear all call counts and promotions for a new session.
pub fn reset() {
    *global()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = ToolPromoter::default();
}

/// Return the process-wide session tool promoter.
#[must_use]
pub fn global() -> &'static Mutex<ToolPromoter> {
    static PROMOTER: OnceLock<Mutex<ToolPromoter>> = OnceLock::new();
    PROMOTER.get_or_init(|| Mutex::new(ToolPromoter::default()))
}

#[cfg(test)]
mod tests {
    use super::{is_promoted, promoted_tools, record_call, reset};
    use serde_json::{Map, Value};
    use std::sync::{Mutex, MutexGuard};

    fn isolated() -> MutexGuard<'static, ()> {
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let guard = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        reset();
        guard
    }

    #[test]
    fn test_initial_state_empty() {
        let _guard = isolated();
        assert!(promoted_tools().is_empty());
    }

    #[test]
    fn test_callgraph_promotes_architecture() {
        let _guard = isolated();
        assert!(record_call("ctx_callgraph", None));
        assert!(is_promoted("ctx_architecture"));
    }

    #[test]
    fn test_five_calls_promotes_session() {
        let _guard = isolated();
        for _ in 0..5 {
            record_call("ctx_read", None);
        }
        assert!(is_promoted("ctx_session"));
        assert!(is_promoted("ctx_knowledge"));
    }

    #[test]
    fn test_is_promoted() {
        let _guard = isolated();
        record_call("ctx_compose", None);
        assert!(is_promoted("ctx_overview"));
        assert!(!is_promoted("ctx_architecture"));
    }

    #[test]
    fn test_reset_clears() {
        let _guard = isolated();
        record_call("ctx_callgraph", None);
        reset();
        assert!(promoted_tools().is_empty());
    }

    #[test]
    fn test_record_returns_true_on_new_promotion() {
        let _guard = isolated();
        assert!(record_call("ctx_callgraph", None));
        assert!(!record_call("ctx_callgraph", None));
    }

    #[test]
    fn search_symbol_and_shell_build_promote_related_tools() {
        let _guard = isolated();
        let search_args = Map::from_iter([("action".to_string(), Value::from("symbol"))]);
        assert!(record_call("ctx_search", Some(&search_args)));
        assert!(is_promoted("ctx_refactor"));

        let shell_args = Map::from_iter([("command".to_string(), Value::from("cargo test"))]);
        assert!(record_call("ctx_shell", Some(&shell_args)));
        assert!(is_promoted("ctx_verify"));
    }
}
