//! Context Control Kernel — unified orchestration over all context stores.

#[allow(dead_code)]
pub(crate) mod a2a_fixes;
#[allow(dead_code)]
pub(crate) mod accounting_fix;
#[allow(dead_code)]
pub(crate) mod activation;
pub(crate) mod activation_e2e;
#[allow(dead_code)]
pub(crate) mod adaptive_bridge;
#[allow(dead_code)]
pub(crate) mod adaptive_hook;
pub(crate) mod airgap_e2e;
#[allow(dead_code)]
pub(crate) mod attribution;
pub(crate) mod bench;
#[allow(dead_code)]
pub(crate) mod bounded;
#[allow(dead_code)]
pub(crate) mod bridge;
pub(crate) mod bridge_e2e;
#[allow(dead_code)]
pub(crate) mod capsule_wire;
pub(crate) mod client_e2e;
#[allow(dead_code)]
pub(crate) mod client_profile;
#[allow(dead_code)]
pub(crate) mod client_wiring;
#[allow(dead_code)]
pub(crate) mod config_bridge;
pub(crate) mod conformance;
#[allow(dead_code)]
pub(crate) mod context_broker;
#[allow(dead_code)]
pub(crate) mod context_dedup;
#[allow(dead_code)]
pub(crate) mod coverage_class;
#[allow(dead_code)]
pub(crate) mod ctx_read_dedup;
#[allow(dead_code)]
pub(crate) mod dashboard_report;
#[allow(dead_code)]
pub(crate) mod dedup_wiring;
#[allow(dead_code)]
pub(crate) mod degradation;
#[allow(dead_code)]
pub(crate) mod enforce;
#[allow(dead_code)]
pub(crate) mod envelope_bridge;
pub(crate) mod envelope_e2e;
pub(crate) mod envelope_wiring;
#[allow(dead_code)]
pub(crate) mod etpao;
#[allow(dead_code)]
pub(crate) mod etpao_live;
#[allow(dead_code)]
pub(crate) mod evidence_hook;
#[allow(dead_code)]
pub(crate) mod evidence_wiring;
#[allow(dead_code)]
pub(crate) mod feedback;
pub(crate) mod feedback_e2e;
#[allow(dead_code)]
pub(crate) mod health;
#[allow(dead_code)]
pub(crate) mod health_api;
#[allow(dead_code)]
pub(crate) mod hotpath_wiring;
#[allow(dead_code)]
pub(crate) mod identity;
pub(crate) mod identity_resolver;
pub(crate) mod integration_e2e;
#[allow(dead_code)]
pub(crate) mod invalidation;
pub(crate) mod kernel_config;
#[allow(dead_code)]
pub(crate) mod knowledge_health;
#[allow(dead_code)]
pub(crate) mod learning;
#[allow(dead_code)]
pub(crate) mod list_tools_opt;
#[allow(dead_code)]
pub(crate) mod live_dashboard;
#[allow(dead_code)]
pub(crate) mod mcp_bridge;
#[allow(dead_code)]
pub(crate) mod mcp_coverage;
pub(crate) mod mcp_e2e;
#[allow(dead_code)]
pub(crate) mod mcp_receipt;
#[allow(dead_code)]
pub(crate) mod mcp_schema_opt;
pub(crate) mod multi_agent_e2e;
#[allow(dead_code)]
pub(crate) mod orchestrator;
#[allow(dead_code)]
pub(crate) mod outcome_signal;
pub(crate) mod perf_benchmark;
#[allow(dead_code)]
pub(crate) mod policy;
#[allow(dead_code)]
pub(crate) mod policy_engine;
pub(crate) mod production_e2e;
#[allow(dead_code)]
pub(crate) mod provider_display;
pub(crate) mod provider_metrics_e2e;
#[allow(dead_code)]
pub(crate) mod provider_parity;
pub(crate) mod provider_traces;
pub(crate) mod providers;
#[allow(dead_code)]
pub(crate) mod proxy_bridge;
pub(crate) mod quality_e2e;
#[allow(dead_code)]
pub(crate) mod receipt_chain;
#[allow(dead_code)]
pub(crate) mod recovery;
#[allow(dead_code)]
pub(crate) mod response_evidence;
#[allow(dead_code)]
pub(crate) mod result_fusion;
#[allow(dead_code)]
pub(crate) mod schema_wiring;
#[allow(dead_code)]
pub(crate) mod shadow;
pub(crate) mod smoke_test;
#[allow(dead_code)]
pub(crate) mod startup;
#[allow(dead_code)]
pub(crate) mod stream_controller;
pub(crate) mod token_envelope;
#[allow(dead_code)]
pub(crate) mod tool_surface;
#[allow(dead_code)]
pub(crate) mod types;
#[allow(dead_code)]
pub(crate) mod usage_normalizer;
pub(crate) mod wiring_e2e;
