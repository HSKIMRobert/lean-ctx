//! Kernel activation configuration and receipt-driven feedback wiring.

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use super::enforce::KernelMode;
use super::feedback::FeedbackCollector;
use super::learning::OutcomeLearner;
use super::types::{ContextReceiptV1, ReceiptOutcome};

const MAX_SUPPLEMENT_TOKENS: usize = 150;

/// Configuration for kernel activation mode and feedback.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActivationConfig {
    /// Kernel operating mode: Shadow (log only), Enforce (apply decisions),
    /// or Explain (log + annotate).
    pub mode: KernelModeConfig,
    /// Whether to track real outcomes (accept/reject) from the LLM.
    pub outcome_tracking: bool,
    /// Hard cap on tokens the kernel may add per request.
    pub max_supplement_tokens: usize,
    /// Enable feedback loop: outcomes → weight updates → better selection.
    pub feedback_loop: bool,
}

/// Serializable kernel operating mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KernelModeConfig {
    /// Log kernel decisions but don't enforce them. Safe default.
    #[default]
    Shadow,
    /// Apply kernel decisions: suppress low-value context, enforce budget.
    Enforce,
    /// Like Shadow but annotate output with kernel reasoning.
    Explain,
}

impl From<KernelModeConfig> for KernelMode {
    fn from(mode: KernelModeConfig) -> Self {
        match mode {
            KernelModeConfig::Shadow => Self::Shadow,
            KernelModeConfig::Enforce => Self::Enforce,
            KernelModeConfig::Explain => Self::Explain,
        }
    }
}

#[derive(Debug, Default, serde::Deserialize)]
struct ConfigFile {
    kernel: Option<KernelOverrides>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct KernelOverrides {
    mode: Option<KernelModeConfig>,
    outcome_tracking: Option<bool>,
    #[serde(alias = "max_supplement")]
    max_supplement_tokens: Option<usize>,
    feedback_loop: Option<bool>,
}

/// Loads kernel activation settings from global and project-local lean-ctx config.
///
/// Missing or malformed files retain safe defaults. Project-local settings in
/// `.lean-ctx.toml` override the global `config.toml` settings.
pub fn load_config(project_root: &str) -> ActivationConfig {
    let mut config = safe_defaults();

    if let Some(path) = crate::core::config::Config::path()
        && let Some(overrides) = read_kernel_overrides(&path)
    {
        apply_overrides(&mut config, &overrides);
    }

    let local_path = crate::core::config::Config::local_path(project_root);
    if let Some(overrides) = read_kernel_overrides(&local_path) {
        apply_overrides(&mut config, &overrides);
    }

    config
}

/// Returns a receipt copy carrying the observed accept/reject outcome.
pub fn record_real_outcome(receipt: &ContextReceiptV1, accepted: bool) -> ContextReceiptV1 {
    let mut recorded = receipt.clone();
    recorded.outcome = if accepted {
        ReceiptOutcome::Accepted
    } else {
        ReceiptOutcome::Rejected
    };
    recorded
}

/// Feeds a known receipt outcome into persisted feedback and provider learning.
///
/// Unknown and partial outcomes carry no binary accept/reject signal and are
/// ignored. Feedback persistence handles unavailable paths without panicking.
pub fn connect_feedback(receipt: &ContextReceiptV1, project_root: &str) {
    if !matches!(
        receipt.outcome,
        ReceiptOutcome::Accepted | ReceiptOutcome::Rejected
    ) {
        return;
    }

    let mut collector = FeedbackCollector::default_for_project(project_root);
    collector.load_weights();

    let mut weights: HashMap<String, f64> = receipt
        .feedback_attribution
        .keys()
        .map(|provider| (provider.clone(), collector.provider_weight(provider)))
        .collect();
    let learner = OutcomeLearner::default_learner();
    let updates = learner.learn_from_receipt(receipt, &weights);
    OutcomeLearner::apply_updates(&mut weights, &updates);

    collector.record_outcome(receipt);
}

/// Returns whether kernel supplementation should run in the configured mode.
pub fn should_supplement(config: &ActivationConfig) -> bool {
    matches!(
        config.mode,
        KernelModeConfig::Shadow | KernelModeConfig::Enforce | KernelModeConfig::Explain
    )
}

/// Returns the bounded per-request token budget for kernel supplementation.
pub fn supplement_budget(config: &ActivationConfig) -> usize {
    config.max_supplement_tokens.min(MAX_SUPPLEMENT_TOKENS)
}

/// Returns whether the selected mode may suppress low-value context.
pub fn should_suppress_in_mode(mode: KernelModeConfig) -> bool {
    matches!(KernelMode::from(mode), KernelMode::Enforce)
}

fn safe_defaults() -> ActivationConfig {
    ActivationConfig {
        mode: KernelModeConfig::Shadow,
        outcome_tracking: false,
        max_supplement_tokens: MAX_SUPPLEMENT_TOKENS,
        feedback_loop: false,
    }
}

fn read_kernel_overrides(path: &Path) -> Option<KernelOverrides> {
    match fs::read_to_string(path) {
        Ok(raw) => match toml::from_str::<ConfigFile>(&raw) {
            Ok(config) => config.kernel,
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "invalid kernel configuration");
                None
            }
        },
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "unable to read kernel configuration");
            None
        }
    }
}

fn apply_overrides(config: &mut ActivationConfig, overrides: &KernelOverrides) {
    if let Some(mode) = overrides.mode {
        config.mode = mode;
    }
    if let Some(enabled) = overrides.outcome_tracking {
        config.outcome_tracking = enabled;
    }
    if let Some(tokens) = overrides.max_supplement_tokens {
        config.max_supplement_tokens = tokens;
    }
    if let Some(enabled) = overrides.feedback_loop {
        config.feedback_loop = enabled;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        ActivationConfig, KernelModeConfig, connect_feedback, load_config, record_real_outcome,
        should_suppress_in_mode, supplement_budget,
    };
    use crate::core::context_kernel::types::{ContextReceiptV1, ReceiptOutcome};

    fn receipt(outcome: ReceiptOutcome) -> ContextReceiptV1 {
        ContextReceiptV1 {
            receipt_id: "receipt-1".to_owned(),
            plan_id: "plan-1".to_owned(),
            delivered_tokens: 10,
            cache_hits: 0,
            cache_misses: 0,
            outcome,
            quality_signals: Vec::new(),
            feedback_attribution: HashMap::new(),
        }
    }

    fn config(mode: KernelModeConfig, max_supplement_tokens: usize) -> ActivationConfig {
        ActivationConfig {
            mode,
            outcome_tracking: false,
            max_supplement_tokens,
            feedback_loop: false,
        }
    }

    #[test]
    fn default_config_is_shadow() {
        let loaded = load_config("/path/that/does/not/exist");
        assert_eq!(loaded.mode, KernelModeConfig::Shadow);
        assert!(!loaded.outcome_tracking);
    }

    #[test]
    fn supplement_budget_capped_at_150() {
        assert_eq!(
            supplement_budget(&config(KernelModeConfig::Enforce, 500)),
            150
        );
    }

    #[test]
    fn real_outcome_sets_rejected() {
        let original = receipt(ReceiptOutcome::Unknown);
        let recorded = record_real_outcome(&original, false);

        assert_eq!(recorded.outcome, ReceiptOutcome::Rejected);
        assert_eq!(original.outcome, ReceiptOutcome::Unknown);
    }

    #[test]
    fn shadow_mode_never_suppresses() {
        assert!(!should_suppress_in_mode(KernelModeConfig::Shadow));
    }

    #[test]
    fn enforce_mode_suppresses() {
        assert!(should_suppress_in_mode(KernelModeConfig::Enforce));
    }

    #[test]
    fn connect_feedback_graceful_on_error() {
        connect_feedback(
            &receipt(ReceiptOutcome::Rejected),
            "/path/that/does/not/exist",
        );
    }
}
