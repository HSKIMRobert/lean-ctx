use std::collections::HashMap;

use rmcp::model::Tool;

use super::tool_trait::McpTool;

/// Central registry mapping tool names to their trait-based handlers.
/// Replaces the match-cascade dispatch for migrated tools while
/// coexisting with the legacy dispatch for tools not yet migrated.
pub struct ToolRegistry {
    tools: HashMap<&'static str, Box<dyn McpTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn McpTool>) {
        self.tools.insert(tool.name(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn McpTool> {
        self.tools.get(name).map(AsRef::as_ref)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Returns MCP Tool definitions for all registered tools.
    /// Used by `list_tools` to expose schemas to clients.
    pub fn tool_defs(&self) -> Vec<Tool> {
        let mut defs: Vec<Tool> = self.tools.values().map(|t| t.tool_def()).collect();
        defs.sort_by(|a, b| a.name.as_ref().cmp(b.name.as_ref()));
        defs
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.tools.keys().copied().collect();
        names.sort_unstable();
        names
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Register all trait-based tools. Called once during server startup.
/// Tools are added here as they are migrated from the legacy dispatch.
pub fn build_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    use crate::tools::registered;
    registry.register(Box::new(registered::ctx_tree::CtxTreeTool));
    registry.register(Box::new(registered::ctx_benchmark::CtxBenchmarkTool));
    registry.register(Box::new(registered::ctx_analyze::CtxAnalyzeTool));
    registry.register(Box::new(registered::ctx_discover::CtxDiscoverTool));
    registry.register(Box::new(registered::ctx_response::CtxResponseTool));
    registry.register(Box::new(registered::ctx_wrapped::CtxWrappedTool));
    registry.register(Box::new(registered::ctx_heatmap::CtxHeatmapTool));
    registry.register(Box::new(registered::ctx_verify::CtxVerifyTool));
    registry.register(Box::new(registered::ctx_outline::CtxOutlineTool));
    registry.register(Box::new(registered::ctx_cost::CtxCostTool));
    registry.register(Box::new(registered::ctx_gain::CtxGainTool));
    registry.register(Box::new(registered::ctx_expand::CtxExpandTool));
    registry.register(Box::new(registered::ctx_routes::CtxRoutesTool));
    registry.register(Box::new(registered::ctx_callers::CtxCallersTool));
    registry.register(Box::new(registered::ctx_callees::CtxCalleesTool));
    registry.register(Box::new(registered::ctx_callgraph::CtxCallgraphTool));
    registry.register(Box::new(registered::ctx_symbol::CtxSymbolTool));
    registry.register(Box::new(registered::ctx_graph_diagram::CtxGraphDiagramTool));
    registry.register(Box::new(
        registered::ctx_discover_tools::CtxDiscoverToolsTool,
    ));
    registry.register(Box::new(registered::ctx_review::CtxReviewTool));
    registry.register(Box::new(registered::ctx_provider::CtxProviderTool));
    registry.register(Box::new(registered::ctx_impact::CtxImpactTool));
    registry.register(Box::new(registered::ctx_architecture::CtxArchitectureTool));
    registry.register(Box::new(registered::ctx_smells::CtxSmellsTool));
    registry.register(Box::new(registered::ctx_pack::CtxPackTool));
    registry.register(Box::new(registered::ctx_index::CtxIndexTool));
    registry.register(Box::new(registered::ctx_artifacts::CtxArtifactsTool));
    registry.register(Box::new(
        registered::ctx_compress_memory::CtxCompressMemoryTool,
    ));

    registry
}
