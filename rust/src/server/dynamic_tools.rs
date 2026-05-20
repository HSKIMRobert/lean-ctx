use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    Core,
    Internal,
    Arch,
    Debug,
    Memory,
    Metrics,
    Session,
}

impl ToolCategory {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "core" => Some(Self::Core),
            "arch" | "architecture" => Some(Self::Arch),
            "debug" | "profiling" => Some(Self::Debug),
            "memory" | "semantic" => Some(Self::Memory),
            "metrics" | "stats" => Some(Self::Metrics),
            "session" => Some(Self::Session),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Internal => "internal",
            Self::Arch => "arch",
            Self::Debug => "debug",
            Self::Memory => "memory",
            Self::Metrics => "metrics",
            Self::Session => "session",
        }
    }
}

#[allow(clippy::match_same_arms)]
pub fn categorize_tool(name: &str) -> ToolCategory {
    match name {
        // Internal: meta/self-referential + automated mechanisms (never exposed)
        "ctx_metrics"
        | "ctx_cost"
        | "ctx_gain"
        | "ctx_radar"
        | "ctx_heatmap"
        | "ctx_feedback"
        | "ctx_intent"
        | "ctx_response"
        | "ctx_discover"
        | "ctx_discover_tools"
        | "ctx_load_tools"
        | "ctx_dedup"
        | "ctx_preload"
        | "ctx_prefetch"
        | "ctx_compress_memory" => ToolCategory::Internal,

        // Core: always visible
        "ctx_read" | "ctx_search" | "ctx_shell" | "ctx_tree" | "ctx_edit" | "ctx_session"
        | "ctx_knowledge" | "ctx_overview" | "ctx_graph" | "ctx_call" | "ctx_compress"
        | "ctx_cache" | "ctx_retrieve" => ToolCategory::Core,

        // Merged tools (redirects in registry, treated as Core for backward compat)
        "ctx_multi_read" | "ctx_smart_read" | "ctx_delta" | "ctx_outline" | "ctx_context" => {
            ToolCategory::Core
        }

        // Arch: on-demand architecture analysis
        "ctx_architecture" | "ctx_impact" | "ctx_callgraph" | "ctx_refactor" | "ctx_symbol"
        | "ctx_routes" | "ctx_smells" | "ctx_index" => ToolCategory::Arch,

        // Debug/Verify: on-demand quality analysis
        "ctx_benchmark" | "ctx_verify" | "ctx_analyze" | "ctx_profile" | "ctx_proof"
        | "ctx_review" => ToolCategory::Debug,

        // Provider is Core: ctx_provider is the gateway to external context
        // (GitHub issues, Jira, Postgres, etc.) and must always be available.
        "ctx_provider" => ToolCategory::Core,

        // Memory: on-demand semantic tools
        "ctx_semantic_search" | "ctx_artifacts" => ToolCategory::Memory,

        // Batch: on-demand batch/PR/sandbox tools
        "ctx_fill" | "ctx_execute" | "ctx_expand" | "ctx_pack" | "ctx_plan" | "ctx_control"
        | "ctx_compile" => ToolCategory::Metrics,

        // Multi-agent: on-demand collaboration
        "ctx_agent" | "ctx_share" | "ctx_task" | "ctx_handoff" | "ctx_workflow" => {
            ToolCategory::Session
        }

        _ => ToolCategory::Core,
    }
}

#[derive(Debug)]
pub struct DynamicToolState {
    active_categories: HashSet<ToolCategory>,
    supports_list_changed: bool,
}

impl Default for DynamicToolState {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicToolState {
    pub fn new() -> Self {
        let mut active = HashSet::new();
        active.insert(ToolCategory::Core);
        active.insert(ToolCategory::Session);
        Self {
            active_categories: active,
            supports_list_changed: false,
        }
    }

    pub fn all_enabled() -> Self {
        let mut active = HashSet::new();
        active.insert(ToolCategory::Core);
        active.insert(ToolCategory::Arch);
        active.insert(ToolCategory::Debug);
        active.insert(ToolCategory::Memory);
        active.insert(ToolCategory::Metrics);
        active.insert(ToolCategory::Session);
        Self {
            active_categories: active,
            supports_list_changed: false,
        }
    }

    pub fn set_supports_list_changed(&mut self, val: bool) {
        self.supports_list_changed = val;
    }

    pub fn supports_list_changed(&self) -> bool {
        self.supports_list_changed
    }

    pub fn load_category(&mut self, cat: ToolCategory) -> bool {
        self.active_categories.insert(cat)
    }

    pub fn unload_category(&mut self, cat: ToolCategory) -> bool {
        if cat == ToolCategory::Core || cat == ToolCategory::Internal {
            return false;
        }
        self.active_categories.remove(&cat)
    }

    pub fn is_tool_active(&self, name: &str) -> bool {
        let cat = categorize_tool(name);
        if cat == ToolCategory::Internal {
            return false;
        }
        if !self.supports_list_changed {
            return true;
        }
        self.active_categories.contains(&cat)
    }

    pub fn active_categories(&self) -> Vec<&'static str> {
        let mut cats: Vec<_> = self
            .active_categories
            .iter()
            .map(ToolCategory::as_str)
            .collect();
        cats.sort_unstable();
        cats
    }

    pub fn all_categories() -> Vec<&'static str> {
        vec!["core", "arch", "debug", "memory", "metrics", "session"]
    }
}

static GLOBAL: OnceLock<Mutex<DynamicToolState>> = OnceLock::new();

pub fn global() -> &'static Mutex<DynamicToolState> {
    GLOBAL.get_or_init(|| Mutex::new(DynamicToolState::new()))
}

pub fn init_all_enabled() {
    let _ = GLOBAL.set(Mutex::new(DynamicToolState::all_enabled()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_tools_always_active() {
        let state = DynamicToolState::new();
        assert!(state.is_tool_active("ctx_read"));
        assert!(state.is_tool_active("ctx_search"));
    }

    #[test]
    fn dynamic_tools_filtered_when_list_changed() {
        let mut state = DynamicToolState::new();
        state.set_supports_list_changed(true);
        assert!(!state.is_tool_active("ctx_benchmark"));
        assert!(!state.is_tool_active("ctx_architecture"));
        assert!(state.is_tool_active("ctx_read"));
    }

    #[test]
    fn load_category_enables_tools() {
        let mut state = DynamicToolState::new();
        state.set_supports_list_changed(true);
        assert!(!state.is_tool_active("ctx_architecture"));
        state.load_category(ToolCategory::Arch);
        assert!(state.is_tool_active("ctx_architecture"));
    }

    #[test]
    fn cannot_unload_core() {
        let mut state = DynamicToolState::new();
        assert!(!state.unload_category(ToolCategory::Core));
    }

    #[test]
    fn all_tools_visible_without_list_changed() {
        let state = DynamicToolState::new();
        assert!(state.is_tool_active("ctx_graph"));
        assert!(!state.is_tool_active("ctx_metrics")); // Internal tools never active
    }

    #[test]
    fn internal_tools_never_active() {
        let state = DynamicToolState::all_enabled();
        assert!(!state.is_tool_active("ctx_metrics"));
        assert!(!state.is_tool_active("ctx_cost"));
        assert!(!state.is_tool_active("ctx_discover_tools"));
        assert!(!state.is_tool_active("ctx_dedup"));
    }

    #[test]
    fn categorize_known_tools() {
        assert_eq!(categorize_tool("ctx_read"), ToolCategory::Core);
        assert_eq!(categorize_tool("ctx_graph"), ToolCategory::Core);
        assert_eq!(categorize_tool("ctx_benchmark"), ToolCategory::Debug);
        assert_eq!(categorize_tool("ctx_semantic_search"), ToolCategory::Memory);
        assert_eq!(categorize_tool("ctx_metrics"), ToolCategory::Internal);
        assert_eq!(categorize_tool("ctx_workflow"), ToolCategory::Session);
    }
}
