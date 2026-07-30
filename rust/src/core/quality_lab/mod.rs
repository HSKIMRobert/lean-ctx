pub(crate) mod calibration;
pub(crate) mod fidelity;
pub(crate) mod orchestrator;

#[cfg(test)]
mod golden_corpus;

pub(crate) use orchestrator::{format_quality_report, run_quality_lab};
