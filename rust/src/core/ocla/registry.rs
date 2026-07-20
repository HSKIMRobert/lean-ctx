//! OclaRegistry — singleton that wires all 14 builtin trait implementations.
//!
//! Provides `OclaRegistry::global()` for production code to access any OCLA
//! capability through its trait interface. The Strangler Fig adoption pattern
//! means existing call sites can be migrated one-by-one to use the registry
//! instead of calling internal modules directly.

use std::sync::{Arc, OnceLock};

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
mod tests {
    use super::*;
    use crate::core::ocla::types::{
        OCLA_API_VERSION, OclaCapability, OclaCapabilityKind, OclaCapabilityStatus,
    };

    fn assert_builtin_capability(capability: OclaCapability, expected: OclaCapabilityKind) {
        assert_eq!(capability.kind, expected);
        assert_eq!(capability.status, OclaCapabilityStatus::Available);
        assert_eq!(capability.api_version, OCLA_API_VERSION);
    }

    macro_rules! builtin_capability_test {
        ($name:ident, $field:ident, $kind:ident) => {
            #[test]
            fn $name() {
                let registry = OclaRegistry::with_builtins();
                assert_builtin_capability(registry.$field.capability(), OclaCapabilityKind::$kind);
            }
        };
    }

    #[test]
    fn global_returns_the_same_registry_instance() {
        let first = OclaRegistry::global();
        let second = OclaRegistry::global();

        assert!(std::ptr::eq(first, second));
    }

    builtin_capability_test!(
        observation_hook_builtin_is_initialized_and_valid,
        observation_hook,
        ObservationHook
    );
    builtin_capability_test!(
        usage_sink_builtin_is_initialized_and_valid,
        usage_sink,
        UsageSink
    );
    builtin_capability_test!(
        metrics_exporter_builtin_is_initialized_and_valid,
        metrics_exporter,
        MetricsExporter
    );
    builtin_capability_test!(
        savings_ledger_builtin_is_initialized_and_valid,
        savings_ledger,
        SavingsLedger
    );
    builtin_capability_test!(
        intent_classifier_builtin_is_initialized_and_valid,
        intent_classifier,
        IntentClassifier
    );
    builtin_capability_test!(
        outcome_tracker_builtin_is_initialized_and_valid,
        outcome_tracker,
        OutcomeTracker
    );
    builtin_capability_test!(
        compression_provider_builtin_is_initialized_and_valid,
        compression_provider,
        CompressionProvider
    );
    builtin_capability_test!(
        response_optimizer_builtin_is_initialized_and_valid,
        response_optimizer,
        ResponseOptimizer
    );
    builtin_capability_test!(
        model_router_builtin_is_initialized_and_valid,
        model_router,
        ModelRouter
    );
    builtin_capability_test!(
        efficiency_analyzer_builtin_is_initialized_and_valid,
        efficiency_analyzer,
        EfficiencyAnalyzer
    );
    builtin_capability_test!(
        config_tuner_builtin_is_initialized_and_valid,
        config_tuner,
        ConfigTuner
    );
    builtin_capability_test!(
        experiment_runner_builtin_is_initialized_and_valid,
        experiment_runner,
        ExperimentRunner
    );
    builtin_capability_test!(
        connector_scheduler_builtin_is_initialized_and_valid,
        connector_scheduler,
        ConnectorScheduler
    );
    builtin_capability_test!(
        agent_gateway_builtin_is_initialized_and_valid,
        agent_gateway,
        AgentGateway
    );

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
}
