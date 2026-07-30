//! Combined savings benchmark study (E-Bench).
//!
//! Four-arm experiment harness: Control / Compress / Route / Combined
//! against standard coding benchmarks (HumanEval, MBPP, SWE-bench).
//! Proves lean-ctx multiplicative cost savings with quality retention.

pub(crate) mod analysis;
pub(crate) mod datasets;
#[allow(dead_code)]
pub(crate) mod experiment;
pub(crate) mod llm_client;
pub(crate) mod metrics;
pub(crate) mod report;
pub(crate) mod runner;
#[allow(dead_code)]
pub(crate) mod sandbox;
pub(crate) mod stats;

pub(crate) use analysis::PublicationAnalysis;
pub(crate) use experiment::StudyConfig;
pub(crate) use runner::run_study;
