// Hard defaults: token-first, output-stable, deterministic.

pub(crate) const READ_MODE_COUNT: f64 = 10.0;

pub(crate) const KNOWLEDGE_RECALL_FACTS_LIMIT: usize = 10;
pub(crate) const KNOWLEDGE_TIMELINE_LIMIT: usize = 25;
pub(crate) const KNOWLEDGE_ROOMS_LIMIT: usize = 25;
pub(crate) const KNOWLEDGE_CROSS_PROJECT_SEARCH_LIMIT: usize = 20;

pub(crate) const KNOWLEDGE_SUMMARY_ROOMS_LIMIT: usize = 10;
pub(crate) const KNOWLEDGE_SUMMARY_FACTS_PER_ROOM_LIMIT: usize = 3;

pub(crate) const KNOWLEDGE_AAAK_ROOMS_LIMIT: usize = 8;
pub(crate) const KNOWLEDGE_AAAK_FACTS_PER_ROOM_LIMIT: usize = 3;

pub(crate) const KNOWLEDGE_PATTERNS_LIMIT: usize = 25;

pub(crate) const KNOWLEDGE_REHYDRATE_LIMIT: usize = 3;

pub(crate) const PROSPECTIVE_REMINDERS_LIMIT: usize = 2;
pub(crate) const PROSPECTIVE_REMINDER_MAX_CHARS: usize = 160;

pub(crate) const INTENTS_PER_SESSION_LIMIT: usize = 50;

// Graph-driven context (budgeted, deterministic)
pub(crate) const GRAPH_CONTEXT_TOKEN_BUDGET: usize = 8000;
pub(crate) const GRAPH_CONTEXT_MAX_FILES: usize = 8;
pub(crate) const GRAPH_CONTEXT_MAX_EDGES: usize = 250;
pub(crate) const GRAPH_CONTEXT_MAX_DEPTH: usize = 2;

// Property graph tools (bounded outputs, deterministic ordering)
pub(crate) const IMPACT_AFFECTED_FILES_LIMIT: usize = 200;

pub(crate) const ARCHITECTURE_OVERVIEW_CLUSTERS_LIMIT: usize = 5;
pub(crate) const ARCHITECTURE_OVERVIEW_LAYERS_LIMIT: usize = 20;
pub(crate) const ARCHITECTURE_OVERVIEW_ENTRYPOINTS_LIMIT: usize = 10;
pub(crate) const ARCHITECTURE_OVERVIEW_CYCLES_LIMIT: usize = 5;
pub(crate) const ARCHITECTURE_CLUSTERS_LIMIT: usize = 25;
pub(crate) const ARCHITECTURE_CLUSTER_FILES_LIMIT: usize = 15;
pub(crate) const ARCHITECTURE_LAYERS_LIMIT: usize = 20;
pub(crate) const ARCHITECTURE_LAYER_FILES_LIMIT: usize = 20;
pub(crate) const ARCHITECTURE_ENTRYPOINTS_LIMIT: usize = 50;
pub(crate) const ARCHITECTURE_CYCLES_LIMIT: usize = 20;
pub(crate) const ARCHITECTURE_MODULE_FILES_LIMIT: usize = 200;

// Knowledge embeddings index (bounded growth)
pub(crate) const KNOWLEDGE_EMBEDDINGS_MAX_FACTS: usize = 2000;
