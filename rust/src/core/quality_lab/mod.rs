pub mod calibration;
pub mod fidelity;
pub mod orchestrator;

#[cfg(test)]
mod golden_corpus;

pub use fidelity::{FidelityAssessment, FidelityClassV1, assess_fidelity};
pub use orchestrator::{QualityGrade, QualityLabReport, format_quality_report, run_quality_lab};
