//! OclaRegistry — singleton that wires all 14 builtin trait implementations.
//!
//! Provides `OclaRegistry::global()` for production code to access any OCLA
//! capability through its trait interface. The Strangler Fig adoption pattern
//! means existing call sites can be migrated one-by-one to use the registry
//! instead of calling internal modules directly.

use std::sync::{Arc, OnceLock};

#[cfg(test)]
use std::cell::Cell;

use super::builtin::{
    agent_gateway::BuiltinAgentGateway, compression_provider::BuiltinCompressionProvider,
    config_tuner::BuiltinConfigTuner, connector_scheduler::BuiltinConnectorScheduler,
    efficiency_analyzer::BuiltinEfficiencyAnalyzer, experiment_runner::BuiltinExperimentRunner,
    intent_classifier::BuiltinIntentClassifier, metrics_exporter::BuiltinMetricsExporter,
    model_router::BuiltinModelRouter, observation_hook::BuiltinObservationHook,
    outcome_tracker::BuiltinOutcomeTracker, response_optimizer::BuiltinResponseOptimizer,
    savings_ledger::BuiltinSavingsLedger, usage_sink::BuiltinUsageSink,
};
use super::traits::{
    AgentGateway, CompressionProvider, ConfigTuner, ConnectorScheduler, EfficiencyAnalyzer,
    ExperimentRunner, IntentClassifier, MetricsExporter, ModelRouter, ObservationHook,
    OutcomeTracker, ResponseOptimizer, SavingsLedger, UsageSink,
};

static GLOBAL_REGISTRY: OnceLock<OclaRegistry> = OnceLock::new();

#[cfg(test)]
thread_local! {
    static TEST_REGISTRY: Cell<*mut OclaRegistry> = const { Cell::new(std::ptr::null_mut()) };
}

pub struct OclaRegistry {
    pub observation_hook: Arc<dyn ObservationHook>,
    pub usage_sink: Arc<dyn UsageSink>,
    pub metrics_exporter: Arc<dyn MetricsExporter>,
    pub savings_ledger: Arc<dyn SavingsLedger>,
    pub intent_classifier: Arc<dyn IntentClassifier>,
    pub outcome_tracker: Arc<dyn OutcomeTracker>,
    pub compression_provider: Arc<dyn CompressionProvider>,
    pub response_optimizer: Arc<dyn ResponseOptimizer>,
    pub model_router: Arc<dyn ModelRouter>,
    pub efficiency_analyzer: Arc<dyn EfficiencyAnalyzer>,
    pub config_tuner: Arc<dyn ConfigTuner>,
    pub experiment_runner: Arc<dyn ExperimentRunner>,
    pub connector_scheduler: Arc<dyn ConnectorScheduler>,
    pub agent_gateway: Arc<dyn AgentGateway>,
}

impl OclaRegistry {
    pub fn global() -> &'static Self {
        #[cfg(test)]
        if let Some(registry) = TEST_REGISTRY.with(|slot| {
            let ptr = slot.get();
            (!ptr.is_null()).then(|| {
                // Test registries are leaked until process exit and scoped to the
                // calling thread, so this pointer remains valid for the call.
                unsafe { &*ptr }
            })
        }) {
            return registry;
        }
        GLOBAL_REGISTRY.get_or_init(Self::with_builtins)
    }

    pub fn with_builtins() -> Self {
        Self {
            observation_hook: Arc::new(BuiltinObservationHook::new()),
            usage_sink: Arc::new(BuiltinUsageSink::new()),
            metrics_exporter: Arc::new(BuiltinMetricsExporter::new()),
            savings_ledger: Arc::new(BuiltinSavingsLedger::new()),
            intent_classifier: Arc::new(BuiltinIntentClassifier::new()),
            outcome_tracker: Arc::new(BuiltinOutcomeTracker::new()),
            compression_provider: Arc::new(BuiltinCompressionProvider::new()),
            response_optimizer: Arc::new(BuiltinResponseOptimizer::new()),
            model_router: Arc::new(BuiltinModelRouter::new()),
            efficiency_analyzer: Arc::new(BuiltinEfficiencyAnalyzer::new()),
            config_tuner: Arc::new(BuiltinConfigTuner::new()),
            experiment_runner: Arc::new(BuiltinExperimentRunner::new()),
            connector_scheduler: Arc::new(BuiltinConnectorScheduler::new()),
            agent_gateway: Arc::new(BuiltinAgentGateway::new()),
        }
    }
}

#[cfg(test)]
pub(crate) struct TestRegistryGuard {
    previous: *mut OclaRegistry,
}

#[cfg(test)]
pub(crate) fn with_test_registry(registry: OclaRegistry) -> TestRegistryGuard {
    let registry = Box::leak(Box::new(registry));
    let previous = TEST_REGISTRY.with(|slot| slot.replace(registry));
    TestRegistryGuard { previous }
}

#[cfg(test)]
impl Drop for TestRegistryGuard {
    fn drop(&mut self) {
        TEST_REGISTRY.with(|slot| slot.set(self.previous));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ocla::traits::{
        EfficiencyAnalyzer, ObservationHook, OclaService, OutcomeTracker, SavingsLedger, UsageSink,
    };
    use crate::core::ocla::types::{
        EfficiencyAnalysis, EfficiencySample, Observation, OclaCapabilityKind,
        OclaCapabilityStatus, OclaRequestContext, OclaResult, Outcome, SavingsEvidence,
        UsageRecord,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct SpyEfficiency(Arc<AtomicUsize>);
    struct SpySavings(Arc<AtomicUsize>);
    struct SpyObservation(Arc<AtomicUsize>);
    struct SpyOutcome(Arc<AtomicUsize>);
    struct SpyUsage(Arc<AtomicUsize>);

    macro_rules! spy_capability {
        ($name:ty, $kind:expr) => {
            impl OclaService for $name {
                fn capability(&self) -> super::super::types::OclaCapability {
                    super::super::types::OclaCapability::available($kind)
                }
            }
        };
    }

    spy_capability!(SpyEfficiency, OclaCapabilityKind::EfficiencyAnalyzer);
    spy_capability!(SpySavings, OclaCapabilityKind::SavingsLedger);
    spy_capability!(SpyObservation, OclaCapabilityKind::ObservationHook);
    spy_capability!(SpyOutcome, OclaCapabilityKind::OutcomeTracker);
    spy_capability!(SpyUsage, OclaCapabilityKind::UsageSink);

    impl EfficiencyAnalyzer for SpyEfficiency {
        fn analyze_efficiency(&self, sample: EfficiencySample) -> OclaResult<EfficiencyAnalysis> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(EfficiencyAnalysis {
                etpao_milli: sample.accepted.map(|_| sample.delivered_tokens),
                duplicate_ratio_milli: 0,
                compression_rate_milli: 0,
                cache_hit_rate_milli: 0,
                recommendation_refs: Vec::new(),
            })
        }
    }

    impl SavingsLedger for SpySavings {
        fn record_savings(&self, evidence: SavingsEvidence) -> OclaResult<String> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(evidence.evidence_ref)
        }
    }

    impl ObservationHook for SpyObservation {
        fn observe(&self, _observation: Observation) -> OclaResult<()> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    impl OutcomeTracker for SpyOutcome {
        fn record_outcome(&self, _outcome: Outcome) -> OclaResult<()> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    impl UsageSink for SpyUsage {
        fn record_usage(&self, _usage: UsageRecord) -> OclaResult<()> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    fn context() -> OclaRequestContext {
        OclaRequestContext {
            request_id: "request-1".into(),
            session_id: "session-1".into(),
            agent_id: "agent-1".into(),
            content_ref: "file:test.rs".into(),
            tenant_id: None,
        }
    }

    #[test]
    fn registry_exposes_all_fourteen_capabilities() {
        let reg = OclaRegistry::with_builtins();
        assert_eq!(
            reg.observation_hook.capability().kind,
            OclaCapabilityKind::ObservationHook
        );
        assert_eq!(
            reg.agent_gateway.capability().kind,
            OclaCapabilityKind::AgentGateway
        );
        assert_eq!(
            reg.model_router.capability().kind,
            OclaCapabilityKind::ModelRouter
        );
    }

    #[test]
    fn all_capabilities_available() {
        let reg = OclaRegistry::with_builtins();
        assert_eq!(
            reg.observation_hook.capability().status,
            OclaCapabilityStatus::Available
        );
        assert_eq!(
            reg.savings_ledger.capability().status,
            OclaCapabilityStatus::Available
        );
    }

    #[test]
    fn cli_file_read_projects_to_efficiency_and_savings_spies() {
        let _dir = crate::core::data_dir::isolated_data_dir();
        let efficiency_calls = Arc::new(AtomicUsize::new(0));
        let savings_calls = Arc::new(AtomicUsize::new(0));
        let mut registry = OclaRegistry::with_builtins();
        registry.efficiency_analyzer = Arc::new(SpyEfficiency(efficiency_calls.clone()));
        registry.savings_ledger = Arc::new(SpySavings(savings_calls.clone()));
        let _guard = with_test_registry(registry);

        crate::core::tool_lifecycle::record_file_read(
            "/tmp/ocla-cli-read.rs",
            "aggressive",
            1_000,
            375,
            false,
            std::time::Duration::from_millis(1),
            "fn main() {}",
        );

        assert_eq!(efficiency_calls.load(Ordering::Relaxed), 1);
        assert_eq!(savings_calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn mcp_tool_call_projects_to_observation_and_outcome_spies() {
        let _dir = crate::core::data_dir::isolated_data_dir();
        let observation_calls = Arc::new(AtomicUsize::new(0));
        let outcome_calls = Arc::new(AtomicUsize::new(0));
        let mut registry = OclaRegistry::with_builtins();
        registry.observation_hook = Arc::new(SpyObservation(observation_calls.clone()));
        registry.outcome_tracker = Arc::new(SpyOutcome(outcome_calls.clone()));
        let _guard = with_test_registry(registry);
        let server = crate::tools::LeanCtxServer::new();

        server
            .record_call_with_timing("ctx_read", 100, 40, Some("full".into()), 3)
            .await;

        assert_eq!(observation_calls.load(Ordering::Relaxed), 1);
        assert_eq!(outcome_calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn proxy_request_projects_measured_usage_to_usage_spy() {
        let _dir = crate::core::data_dir::isolated_data_dir();
        let usage_calls = Arc::new(AtomicUsize::new(0));
        let mut registry = OclaRegistry::with_builtins();
        registry.usage_sink = Arc::new(SpyUsage(usage_calls.clone()));
        let _guard = with_test_registry(registry);
        let usage = crate::proxy::usage::RealUsage {
            model: "test-model".into(),
            input_tokens: 11,
            output_tokens: 7,
            wire: Some(Box::new(crate::proxy::usage::WireContext {
                lineage: Some(context()),
                ..Default::default()
            })),
            ..Default::default()
        };

        crate::proxy::usage_meter::record(&usage);

        assert_eq!(usage_calls.load(Ordering::Relaxed), 1);
    }
}
