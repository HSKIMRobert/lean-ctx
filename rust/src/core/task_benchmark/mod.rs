//! Reproducible task-score benchmark framework (#1328).
//!
//! Runs a fixed set of coding tasks through multiple compression configurations
//! (stock / standard / aggressive) with repeated runs for statistical confidence.
//! Measures both token savings AND output quality to ensure compression never
//! degrades agent performance.

pub(crate) mod config;
pub(crate) mod fixtures;
pub(crate) mod report;
pub(crate) mod runner;
