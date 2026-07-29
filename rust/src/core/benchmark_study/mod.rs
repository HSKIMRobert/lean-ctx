//! Combined savings benchmark study (E-Bench).
//!
//! Four-arm experiment harness: Control / Compress / Route / Combined
//! against standard coding benchmarks (HumanEval, MBPP, SWE-bench).
//! Proves lean-ctx multiplicative cost savings with quality retention.

pub mod analysis;
pub mod datasets;
pub mod experiment;
pub mod llm_client;
pub mod metrics;
pub mod report;
pub mod runner;
pub mod sandbox;
pub mod stats;

pub use analysis::PublicationAnalysis;
pub use experiment::{Arm, FourArmExperiment, StudyConfig};
pub use report::StudyReport;
pub use runner::run_study;
