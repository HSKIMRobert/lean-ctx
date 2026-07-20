//! Routing off-vs-on savings proof (enterprise#21).
//!
//! Answers "does the active router (enterprise#13) produce *real*, auditable
//! savings?" the same way the context A/B answers the quality question —
//! deterministically, at real list prices, over real queries:
//!
//! * **off-arm**: every task is served by the model the client requested.
//! * **on-arm**: the task's last user query runs through the *production*
//!   classifier ([`classify`] → [`route_intent`]) and the configured
//!   [`RoutingRules`] — exactly the logic the proxy applies in-flight — and is
//!   priced at the model the router selects.
//!
//! The savings claim is a pure **rate-card delta**: `input_rate(requested) −
//! input_rate(serving)` per routed task, priced from the shared
//! [`ModelPricing`] table (real provider list prices; enterprise#14). No token
//! counts are invented — absolute USD amounts come from the usage ledger
//! (enterprise#19), which applies the same formula to *measured*
//! `usage_events` rows (`routed_from` × real input tokens). This eval proves
//! the mechanism and the classification distribution; the ledger supplies the
//! volumes.
//!
//! Everything here is a deterministic function of (suite, rules, pricing
//! table): the classifier is lexical, the price table is embedded, and the
//! report digest is byte-stable (#498) — so the artifact is reproducible
//! evidence, not a demo.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::core::config::{RoutingRules, parse_route_target};
use crate::core::gain::model_pricing::{ModelPricing, PricingMatchKind};
use crate::core::intent_engine::{classify, route_intent};
use crate::core::ocla::types::{ExperimentRequest, ExperimentResult};

use super::suite::EvalSuite;

/// Configuration for one routing off-vs-on run.
#[derive(Debug, Clone)]
pub struct RoutingEvalConfig {
    /// The model the off-arm assumes every request targets — the org's
    /// day-to-day default (e.g. the counterfactual `reference_model`).
    pub requested_model: String,
    /// The rule set under test — normally the deployment's `[proxy.routing]`.
    pub rules: RoutingRules,
}

/// One task's routing decision + rate delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingTaskRecord {
    pub task_id: String,
    /// Intent tier the production classifier assigned (`fast|standard|premium`).
    pub tier: String,
    /// Model serving the on-arm (= `requested` when the router kept it).
    pub serving_model: String,
    /// True when the router changed the model (alias or tier hit).
    pub routed: bool,
    /// List input rate (USD/MTok) of the requested model.
    pub requested_input_rate: f64,
    /// List input rate (USD/MTok) of the serving model.
    pub serving_input_rate: f64,
    /// Rate-card saving per 1M input tokens for this task (0 when kept).
    pub input_rate_saving_per_mtok: f64,
}

/// Deterministic off-vs-on routing report — the `savings_ledger`'s
/// attribution witness for the ROUTE mechanism.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingEvalReport {
    pub suite: String,
    pub requested_model: String,
    pub records: Vec<RoutingTaskRecord>,
    pub routed_count: usize,
    pub kept_count: usize,
    /// Tasks classified premium that the rules downgraded anyway. The gate
    /// requires 0: premium work is never silently downgraded (enterprise#13).
    pub premium_downgrades: usize,
    /// Mean rate saving per 1M input tokens across *all* tasks (kept = 0).
    pub mean_input_rate_saving_per_mtok: f64,
}

impl RoutingEvalReport {
    /// Canonical JSON for artifacts and digests.
    ///
    /// # Panics
    /// Only if serde serialization of the report itself fails (plain data).
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("routing report serializes")
    }

    /// Byte-stable digest over the canonical JSON (#498).
    #[must_use]
    pub fn determinism_digest(&self) -> String {
        super::sha256_hex(self.to_json().as_bytes())
    }

    /// True when the safety gate holds: routing never downgraded premium work.
    #[must_use]
    pub fn gate_passes(&self) -> bool {
        self.premium_downgrades == 0
    }

    /// Human-readable side-by-side summary.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "routing off-vs-on — suite '{}', requested model '{}'",
            self.suite, self.requested_model
        );
        let _ = writeln!(
            out,
            "{:<28} {:<9} {:<26} {:>12}",
            "task", "tier", "serving model", "Δ USD/MTok-in"
        );
        for r in &self.records {
            let _ = writeln!(
                out,
                "{:<28} {:<9} {:<26} {:>12.3}",
                r.task_id, r.tier, r.serving_model, r.input_rate_saving_per_mtok
            );
        }
        let _ = writeln!(
            out,
            "\nrouted {}/{} tasks · mean saving {:.3} USD per 1M input tokens · premium downgrades: {}",
            self.routed_count,
            self.routed_count + self.kept_count,
            self.mean_input_rate_saving_per_mtok,
            self.premium_downgrades
        );
        out
    }
}

/// The ROUTE-mechanism attribution formula shared with the savings ledger
/// (enterprise#19): USD saved on `input_tokens` by serving `serving` instead
/// of `requested`, at list input rates. Negative deltas (an upgrade) count as
/// negative savings — the ledger must not hide regressions.
#[must_use]
pub fn routing_saving_usd(
    pricing: &ModelPricing,
    requested: &str,
    serving: &str,
    input_tokens: u64,
) -> f64 {
    let from = pricing.quote(Some(requested)).cost.input_per_m;
    let to = pricing.quote(Some(serving)).cost.input_per_m;
    #[allow(clippy::cast_precision_loss)]
    let tokens = input_tokens as f64;
    (from - to) / 1_000_000.0 * tokens
}

/// Runs the routing off-vs-on comparison over a suite's real task prompts.
///
/// Mirrors the proxy's decision order (`proxy::routing::route_request`):
/// alias on the requested model first, then the intent tier of the query.
/// Unknown/unpriced targets keep the task on the requested model — the eval
/// must not claim savings the proxy would not realize.
///
/// # Errors
/// When the rule set is inactive (nothing to evaluate) or the suite is empty.
pub fn run_routing_eval(
    suite: &EvalSuite,
    suite_name: &str,
    pricing: &ModelPricing,
    cfg: &RoutingEvalConfig,
) -> anyhow::Result<RoutingEvalReport> {
    if !cfg.rules.is_active() {
        anyhow::bail!(
            "routing rules are inactive (enabled + at least one alias/tier required) — \
             configure [proxy.routing] or pass explicit rules"
        );
    }
    if suite.tasks.is_empty() {
        anyhow::bail!("suite has no tasks");
    }

    let requested_quote = pricing.quote(Some(&cfg.requested_model));
    let mut records = Vec::with_capacity(suite.tasks.len());
    let mut premium_downgrades = 0usize;

    for task in &suite.tasks {
        let query = task.query();
        let classification = classify(query);
        let tier = route_intent(query, &classification).model_tier;
        let tier_label = tier.as_str().to_string();

        // Alias first, then tier — the proxy's exact precedence.
        let target = cfg
            .rules
            .aliases
            .get(&cfg.requested_model)
            .cloned()
            .or_else(|| {
                cfg.rules
                    .tiers
                    .get(&tier_label)
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
            });

        let serving_model = target
            .as_deref()
            .and_then(parse_route_target)
            .map(|(_, model)| model.to_string())
            .filter(|m| m != &cfg.requested_model)
            // Unpriced target = the fallback quote → no provable saving; keep.
            .filter(|m| pricing.quote(Some(m)).match_kind != PricingMatchKind::Fallback);

        let routed = serving_model.is_some();
        if routed && tier_label == "premium" {
            premium_downgrades += 1;
        }
        let serving_model = serving_model.unwrap_or_else(|| cfg.requested_model.clone());
        let serving_rate = pricing.quote(Some(&serving_model)).cost.input_per_m;

        records.push(RoutingTaskRecord {
            task_id: task.id.clone(),
            tier: tier_label,
            serving_model,
            routed,
            requested_input_rate: requested_quote.cost.input_per_m,
            serving_input_rate: serving_rate,
            input_rate_saving_per_mtok: requested_quote.cost.input_per_m - serving_rate,
        });
    }

    let routed_count = records.iter().filter(|r| r.routed).count();
    #[allow(clippy::cast_precision_loss)]
    let mean = records
        .iter()
        .map(|r| r.input_rate_saving_per_mtok)
        .sum::<f64>()
        / records.len() as f64;

    Ok(RoutingEvalReport {
        suite: suite_name.to_string(),
        requested_model: cfg.requested_model.clone(),
        records,
        routed_count,
        kept_count: suite.tasks.len() - routed_count,
        premium_downgrades,
        mean_input_rate_saving_per_mtok: mean,
    })
}

/// Production OCLA callsite for the routing A/B experiment.
///
/// `experiment_ref` identifies the NDJSON suite selected by the caller. The
/// report digest becomes the outcome ref, so the OCLA result points at the
/// exact deterministic evaluation instead of fabricating a completion token.
pub fn run_routing_experiment(
    request: &ExperimentRequest,
    requested_model: &str,
    rules: &RoutingRules,
    pricing: &ModelPricing,
) -> anyhow::Result<ExperimentResult> {
    let suite_path = Path::new(&request.experiment_ref);
    let suite = EvalSuite::load(suite_path)?;
    let suite_name = suite_path.file_name().map_or_else(
        || request.experiment_ref.clone(),
        |name| name.to_string_lossy().into_owned(),
    );
    let report = run_routing_eval(
        &suite,
        &suite_name,
        pricing,
        &RoutingEvalConfig {
            requested_model: requested_model.to_string(),
            rules: rules.clone(),
        },
    )?;

    Ok(ExperimentResult {
        experiment_ref: request.experiment_ref.clone(),
        outcome_ref: format!(
            "outcome:{}:{}",
            request.experiment_ref,
            report.determinism_digest()
        ),
        rollback_ref: Some(format!("rollback:{}", request.cohort_ref)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suite_with(prompts: &[(&str, &str)]) -> (tempfile::TempDir, EvalSuite) {
        let root = tempfile::tempdir().unwrap();
        let ws = root.path().join("corpus");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("readme.md"), "fixture corpus").unwrap();
        let raw = prompts
            .iter()
            .map(|(id, prompt)| {
                format!(
                    r#"{{"id":"{id}","domain":"qa","prompt":"{prompt}","workspace":"corpus","answers":["x"]}}"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let suite = EvalSuite::parse(&raw, root.path().to_path_buf()).unwrap();
        (root, suite)
    }

    fn rules(tiers: &[(&str, &str)]) -> RoutingRules {
        RoutingRules {
            enabled: Some(true),
            aliases: std::collections::BTreeMap::default(),
            tiers: tiers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn experiment_request(suite: &std::path::Path) -> ExperimentRequest {
        ExperimentRequest {
            context: crate::core::ocla::types::OclaRequestContext {
                request_id: "request-1".into(),
                session_id: "session-1".into(),
                agent_id: "agent-test".into(),
                content_ref: "ref:test".into(),
                tenant_id: None,
            },
            experiment_ref: suite.to_string_lossy().into_owned(),
            cohort_ref: "cohort:treatment".into(),
        }
    }

    #[test]
    fn off_vs_on_routes_cheap_tiers_and_never_premium() {
        let (_root, suite) = suite_with(&[
            (
                "explore-q",
                "how does the session cache work in this project?",
            ),
            (
                "premium-gen",
                "implement a new distributed lock manager with leader election and fencing tokens",
            ),
        ]);
        let cfg = RoutingEvalConfig {
            requested_model: "claude-opus-4.5".into(),
            rules: rules(&[("fast", "foundry:Phi-4"), ("standard", "foundry:Phi-4")]),
        };
        let pricing = ModelPricing::embedded();
        let report = run_routing_eval(&suite, "fixture", &pricing, &cfg).unwrap();

        assert!(report.gate_passes(), "premium must never be downgraded");
        assert_eq!(report.routed_count, 1, "the explore query routes");
        let routed = report.records.iter().find(|r| r.routed).unwrap();
        // claude-opus-4.5 $5.00/MTok − phi-4 $0.125/MTok = $4.875 per MTok input.
        assert!((routed.input_rate_saving_per_mtok - 4.875).abs() < 1e-9);
        let premium = &report.records[1];
        assert_eq!(premium.tier, "premium");
        assert!(!premium.routed);
        assert_eq!(premium.input_rate_saving_per_mtok, 0.0);

        // Byte-stable evidence (#498): identical inputs → identical digest.
        let again = run_routing_eval(&suite, "fixture", &pricing, &cfg).unwrap();
        assert_eq!(report.determinism_digest(), again.determinism_digest());
    }

    #[test]
    fn unpriced_target_claims_no_saving() {
        let (_root, suite) = suite_with(&[("q", "how does the config loader work?")]);
        let cfg = RoutingEvalConfig {
            requested_model: "claude-opus-4.5".into(),
            rules: rules(&[
                ("fast", "foundry:totally-unknown-model"),
                ("standard", "foundry:totally-unknown-model"),
            ]),
        };
        let report = run_routing_eval(&suite, "s", &ModelPricing::embedded(), &cfg).unwrap();
        assert_eq!(report.routed_count, 0, "unpriced target must not route");
        assert_eq!(report.mean_input_rate_saving_per_mtok, 0.0);
    }

    #[test]
    fn inactive_rules_error_instead_of_empty_claim() {
        let (_root, suite) = suite_with(&[("q", "anything")]);
        let cfg = RoutingEvalConfig {
            requested_model: "gpt-5.4".into(),
            rules: RoutingRules::default(),
        };
        assert!(run_routing_eval(&suite, "s", &ModelPricing::embedded(), &cfg).is_err());
    }

    #[test]
    fn ledger_formula_prices_measured_tokens() {
        let pricing = ModelPricing::embedded();
        // 2M input tokens routed opus→phi-4: 2 × (5.00 − 0.125) = 9.75 USD.
        let usd = routing_saving_usd(&pricing, "claude-opus-4.5", "phi-4", 2_000_000);
        assert!((usd - 9.75).abs() < 1e-9);
        // Upgrades are negative savings — never hidden.
        assert!(routing_saving_usd(&pricing, "phi-4", "claude-opus-4.5", 1_000_000) < 0.0);
    }

    #[test]
    fn ocla_adapter_returns_evaluation_digest_and_rollback_ref() {
        let root = tempfile::tempdir().unwrap();
        let ws = root.path().join("corpus");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("readme.md"), "fixture corpus").unwrap();
        let suite_path = root.path().join("suite.ndjson");
        std::fs::write(
            &suite_path,
            r#"{"id":"q","domain":"qa","prompt":"how does config work?","workspace":"corpus","answers":["config"]}"#,
        )
        .unwrap();

        let result = run_routing_experiment(
            &experiment_request(&suite_path),
            "claude-opus-4.5",
            &rules(&[("standard", "foundry:Phi-4")]),
            &ModelPricing::embedded(),
        )
        .unwrap();

        assert!(result.outcome_ref.starts_with("outcome:"));
        assert_eq!(
            result.rollback_ref.as_deref(),
            Some("rollback:cohort:treatment")
        );
    }

    #[test]
    fn ocla_adapter_propagates_inactive_routing_rules() {
        let (_root, suite) = suite_with(&[("q", "how does config work?")]);
        let suite_path = suite.dir.join("suite.ndjson");
        std::fs::write(
            &suite_path,
            r#"{"id":"q","domain":"qa","prompt":"how does config work?","workspace":"corpus","answers":["config"]}"#,
        )
        .unwrap();
        let request = experiment_request(&suite_path);
        let error = run_routing_experiment(
            &request,
            "claude-opus-4.5",
            &RoutingRules::default(),
            &ModelPricing::embedded(),
        );
        assert!(error.is_err());
    }
}
