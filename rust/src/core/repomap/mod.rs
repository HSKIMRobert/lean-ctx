//! Repo map: PageRank-based symbol importance ranking across a codebase.
//!
//! Provides a ranked view of the most structurally important symbols,
//! personalized by session context (recent files, focus files).

pub(crate) mod budget;
pub(crate) mod graph;
pub(crate) mod ranking;

pub(crate) use budget::fit_to_budget;
pub(crate) use graph::RepoGraph;
pub(crate) use ranking::rank_symbols;
