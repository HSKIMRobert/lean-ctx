//! Reproducible task-score benchmark framework (#1328).
//!
//! Runs a fixed set of coding tasks through multiple compression configurations
//! (stock / standard / aggressive) with repeated runs for statistical confidence.
//! Measures both token savings AND output quality to ensure compression never
//! degrades agent performance.

pub mod config;
pub mod fixtures;
pub mod report;
pub mod runner;

pub use config::BenchConfig;
pub use fixtures::TaskFixture;
pub use report::BenchReport;
pub use runner::run_benchmark;
