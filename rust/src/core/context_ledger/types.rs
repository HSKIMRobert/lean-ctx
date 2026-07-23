use serde::{Deserialize, Serialize};

use crate::core::context_field::{
    ContextItemId, ContextKind, ContextState, Provenance, ViewCosts, ViewKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextLedger {
    pub window_size: usize,
    pub entries: Vec<LedgerEntry>,
    pub total_tokens_sent: usize,
    pub total_tokens_saved: usize,
    #[serde(skip)]
    pub(super) last_flush: Option<std::time::Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub path: String,
    pub mode: String,
    pub original_tokens: usize,
    pub sent_tokens: usize,
    pub timestamp: i64,
    #[serde(default)]
    pub id: Option<ContextItemId>,
    #[serde(default)]
    pub kind: Option<ContextKind>,
    #[serde(default)]
    pub source_hash: Option<String>,
    #[serde(default)]
    pub state: Option<ContextState>,
    #[serde(default)]
    pub phi: Option<f64>,
    #[serde(default)]
    pub view_costs: Option<ViewCosts>,
    #[serde(default)]
    pub active_view: Option<ViewKind>,
    #[serde(default)]
    pub provenance: Option<Provenance>,
    /// How many times this item has been (re)read into context. Drives the
    /// "high tokens + low recent use" eviction-candidate heuristic.
    #[serde(default)]
    pub access_count: u32,
}

#[derive(Debug, Clone)]
pub struct ContextPressure {
    pub utilization: f64,
    pub remaining_tokens: usize,
    pub entries_count: usize,
    pub recommendation: PressureAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureAction {
    NoAction,
    SuggestCompression,
    ForceCompression,
    EvictLeastRelevant,
}

/// Outcome of resolving a user-supplied target against the ledger (#715).
/// Entries store absolute canonical paths while users, hints and the
/// dashboard supply relative paths or basenames — exact matching alone made
/// `evict` a no-op ("Evicted 0/1").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerResolution {
    /// Exactly one entry matched (index into `entries`).
    Unique(usize),
    /// Several entries share the suffix — the matched paths, for diagnostics.
    Ambiguous(Vec<String>),
    NotFound,
}

/// Per-target eviction outcome (#715): carries the canonical resolved path so
/// callers can report precisely what happened and write overlays against the
/// path the ledger actually stores.
#[derive(Debug, Clone)]
pub struct EvictOutcome {
    pub target: String,
    pub resolved: Option<String>,
    pub ambiguous: Vec<String>,
}
