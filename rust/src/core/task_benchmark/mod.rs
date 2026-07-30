//! Reproducible task-score benchmark framework (#1328).
//!
//! Runs a fixed set of coding tasks through multiple compression configurations
//! (stock / standard / aggressive) with repeated runs for statistical confidence.
//! Measures both token savings AND output quality to ensure compression never
//! degrades agent performance.

#[allow(dead_code)]
pub(crate) mod config;
#[allow(dead_code)]
pub(crate) mod fixtures;
pub(crate) mod report;
pub(crate) mod runner;
