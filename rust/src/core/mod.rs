// ---------------------------------------------------------------------------
// Domain: Compression
// ---------------------------------------------------------------------------
#[allow(dead_code)]
pub(crate) mod adaptive_chunking;
#[allow(dead_code)]
pub(crate) mod adaptive_compression;
pub mod addons;
pub(crate) mod aggressiveness;
pub mod attention_context;
pub(crate) mod auto_capture;
pub(crate) mod auto_findings;
#[allow(dead_code)]
pub(crate) mod codebook;
#[cfg(target_os = "macos")]
pub(crate) mod codesign;
#[allow(dead_code)]
pub mod cognitive;
pub(crate) mod compress_preview;
pub(crate) mod compression_safety;
pub mod compressor;
#[cfg(feature = "experimental")]
pub(crate) mod context_budget;
#[allow(dead_code, unreachable_pub)]
pub mod context_prefetch;
pub(crate) mod datadog_push;
pub mod entropy;
#[allow(dead_code)]
pub(crate) mod etpao;
pub mod eval_ab;
pub mod eval_harness;
pub(crate) mod extractive;
pub mod finops_export;
#[allow(dead_code)]
pub(crate) mod html_crush;
#[allow(dead_code, unused_imports, unreachable_pub)]
pub mod ib;
#[allow(dead_code)]
pub(crate) mod information_bottleneck;
#[allow(dead_code)]
pub(crate) mod json_crush;
#[allow(dead_code)]
pub(crate) mod json_sample;
pub(crate) mod markdown_compact;
#[allow(dead_code)]
pub(crate) mod output_sanitizer;
pub mod policy;
#[allow(dead_code)]
pub(crate) mod pop_pruning;
pub mod predictive_coding;
pub mod predictive_prefetch;
#[allow(dead_code)]
pub(crate) mod preservation;
#[allow(dead_code)]
pub(crate) mod process_guard;
#[allow(dead_code)]
pub(crate) mod progressive_compression;
pub(crate) mod protect;
#[allow(dead_code)]
pub(crate) mod rabin_karp;
#[allow(dead_code)]
pub(crate) mod relevance_tracker;
pub mod rule_artifacts;
pub(crate) mod rule_discovery;
#[allow(dead_code)]
pub(crate) mod rule_scorer;
#[cfg(feature = "experimental")]
pub(crate) mod rule_staleness;
pub mod rules_canonical;
#[allow(dead_code)]
pub(crate) mod rules_channel;
pub(crate) mod rules_overhead;
pub(crate) mod rules_sections;
pub(crate) mod rules_validation;
pub(crate) mod runtime_flags;
pub mod sidecar_transport;
#[allow(dead_code)]
pub(crate) mod structural_tokenizer;
pub(crate) mod structured_read;
#[allow(dead_code)]
pub(crate) mod tabular_crush;
#[allow(dead_code, unreachable_pub)]
pub mod verbosity;
#[allow(dead_code, unreachable_pub)]
pub mod wasserstein;
#[allow(dead_code)]
pub(crate) mod yaml_crush;

// ---------------------------------------------------------------------------
// Domain: Memory
// ---------------------------------------------------------------------------
#[allow(dead_code)]
pub mod anti_interrupt;
#[allow(dead_code)]
pub(crate) mod episodic_memory;
#[allow(dead_code)]
pub(crate) mod interrupt;
#[allow(dead_code)]
pub(crate) mod memory_archive;
pub(crate) mod memory_boundary;
pub(crate) mod memory_capacity;
pub(crate) mod memory_consolidation;
#[allow(dead_code)]
pub(crate) mod memory_guard;
#[allow(dead_code)]
pub(crate) mod memory_lifecycle;
pub mod memory_policy;
pub(crate) mod memory_salience;
#[allow(dead_code)]
pub mod memory_scheduler;
pub mod multiscale_index;
#[allow(dead_code)]
pub(crate) mod procedural_memory;
pub(crate) mod prospective_memory;

// ---------------------------------------------------------------------------
// Domain: Graph
// ---------------------------------------------------------------------------
#[allow(dead_code)]
pub(crate) mod call_graph;
#[allow(dead_code)]
pub(crate) mod community;
#[allow(dead_code)]
pub(crate) mod gamma_cover;
pub(crate) mod graph_analysis;
pub mod graph_context;
pub(crate) mod graph_coordinator;
pub(crate) mod graph_enricher;
#[allow(dead_code, unreachable_pub)]
pub mod graph_expand;
pub mod graph_export;
#[allow(dead_code)]
pub(crate) mod graph_features;
pub mod graph_index;
pub(crate) mod graph_parity;
pub mod graph_provider;
pub(crate) mod pagerank;
pub mod property_graph;
pub(crate) mod repomap;

// ---------------------------------------------------------------------------
// Domain: Context
// ---------------------------------------------------------------------------
#[allow(dead_code)]
pub(crate) mod context_artifacts;
pub mod context_column;
pub(crate) mod context_compiler;
#[allow(dead_code)]
pub(crate) mod context_deficit;
pub mod context_field;
#[allow(dead_code)]
pub(crate) mod context_handles;
pub mod context_ir;
pub(crate) mod context_kernel;
pub mod context_ledger;
pub(crate) mod context_lint;
pub mod context_os;
#[allow(dead_code)]
pub(crate) mod context_overhead;
pub mod context_overlay;
pub(crate) mod context_package;
pub mod context_policies;
pub(crate) mod context_proof;
#[allow(dead_code)]
pub(crate) mod context_proof_v2;
pub mod context_radar;
pub(crate) mod context_snapshot;
pub mod cross_source_edges;
pub mod cross_source_hints;

// ---------------------------------------------------------------------------
// Domain: Knowledge
// ---------------------------------------------------------------------------
#[allow(dead_code)]
pub(crate) mod claim_extractor;
pub(crate) mod cognition_loop;
pub(crate) mod cognition_scheduler;
pub mod knowledge;
pub(crate) mod knowledge_bootstrap;
#[allow(dead_code)]
pub(crate) mod knowledge_bridge;
pub mod knowledge_embedding;
pub mod knowledge_provider_extract;
pub mod knowledge_relations;

// ---------------------------------------------------------------------------
// Domain: Search & Retrieval
// ---------------------------------------------------------------------------
#[allow(dead_code)]
pub(crate) mod bm25_cache;
pub mod bm25_index;
#[allow(dead_code)]
pub(crate) mod content_cache;
pub mod content_chunk;
pub(crate) mod context_packing;
#[allow(dead_code)]
pub(crate) mod cooccurrence;
#[allow(dead_code)]
pub(crate) mod dense_backend;
#[allow(dead_code)]
pub(crate) mod embedding_index;
pub(crate) mod embedding_quant;
pub mod embeddings;
#[allow(dead_code)]
pub(crate) mod energy;
pub mod hybrid_search;
#[cfg(feature = "pgvector")]
pub(crate) mod pgvector_store;
#[cfg(feature = "qdrant")]
pub(crate) mod qdrant_store;
pub mod search_reranking;
#[allow(dead_code)]
pub(crate) mod semantic_cache;
#[allow(dead_code)]
pub(crate) mod semantic_chunks;
pub(crate) mod splade_retrieval;
pub mod spreading_activation;

// ---------------------------------------------------------------------------
// Domain: Session & Handoff
// ---------------------------------------------------------------------------
pub(crate) mod ccp_session_bundle;
pub(crate) mod handoff_ledger;
pub(crate) mod handoff_transfer_bundle;
pub mod session;
pub(crate) mod session_diff;
pub(crate) mod session_summary;
pub(crate) mod skillify;

// ---------------------------------------------------------------------------
// Domain: Attention & Placement
// ---------------------------------------------------------------------------
#[allow(dead_code)]
pub(crate) mod attention_layout_driver;
pub mod attention_model;
#[allow(dead_code)]
pub(crate) mod attention_placement;
#[allow(dead_code)]
pub(crate) mod litm;

// ---------------------------------------------------------------------------
// Domain: Neural / ML
// ---------------------------------------------------------------------------
#[allow(dead_code)]
pub(crate) mod neural;
// ORT runtime glue links against the `ort` crate, which is only pulled in by the
// `embeddings` or `neural` features. On platforms ORT does not support (e.g.
// FreeBSD, see #586) these features are disabled, so the modules must be gated
// to keep the build clean without them.
#[cfg(any(feature = "embeddings", feature = "neural"))]
pub(crate) mod ort_environment;
#[cfg(any(feature = "embeddings", feature = "neural"))]
pub(crate) mod ort_execution_providers;

// ---------------------------------------------------------------------------
// Domain: Patterns & Shell
// ---------------------------------------------------------------------------
pub mod patterns;

// ---------------------------------------------------------------------------
// Domain: Agents & A2A
// ---------------------------------------------------------------------------
pub mod a2a;
pub(crate) mod a2a_transport;
pub(crate) mod agent_identity;
pub(crate) mod agent_runtime_env;
pub(crate) mod agents;
pub(crate) mod autonomy;
#[allow(dead_code)]
pub(crate) mod autonomy_drivers;

// ---------------------------------------------------------------------------
// Domain: Adaptive & Scoring
// ---------------------------------------------------------------------------
pub(crate) mod adaptive;
pub(crate) mod adaptive_mode_policy;
#[allow(dead_code)]
pub(crate) mod adaptive_thresholds;
pub mod auto_mode_resolver;
#[allow(dead_code)]
pub(crate) mod bandit;
#[allow(dead_code)]
pub(crate) mod litm_calibration;
#[allow(dead_code)]
pub(crate) mod mode_predictor;
pub(crate) mod model_registry;
pub mod task_relevance;
#[allow(dead_code)]
pub(crate) mod token_calibration;

// ---------------------------------------------------------------------------
// Domain: Diagnostics & Quality
// ---------------------------------------------------------------------------
pub mod anomaly;
pub(crate) mod benchmark;
pub mod benchmark_compare;
pub(crate) mod benchmark_study;
/// Commercial-plane billing substrate (`billing-plane-v1`): plans, entitlements,
/// and usage metering derived from the signed savings ledger. Never gates local.
pub mod billing;
#[allow(dead_code)]
pub(crate) mod code_health;
#[allow(dead_code)]
pub(crate) mod cognitive_load;
pub mod conformance;
pub mod contracts;
pub(crate) mod cyclomatic;
pub mod degradation_policy;
pub mod loop_detection;
pub(crate) mod output_verification;
#[allow(dead_code)]
pub(crate) mod quality;
pub(crate) mod quality_lab;
pub(crate) mod safety_needles;
pub mod scorecard;
pub mod setup_report;
pub(crate) mod slo;
pub(crate) mod slow_log;
pub(crate) mod smells;
pub(crate) mod subagent_contract;
#[allow(dead_code)]
pub(crate) mod surprise;
pub(crate) mod verification_observability;

// ---------------------------------------------------------------------------
// Domain: Config & Infrastructure
// ---------------------------------------------------------------------------
pub mod active_inference;
#[allow(dead_code)]
pub(crate) mod agent_attribution;
#[allow(dead_code)]
pub(crate) mod agent_budget;
#[allow(dead_code)]
pub(crate) mod agent_lease;
pub mod anchor;
pub(crate) mod ann_cache;
pub(crate) mod atomic_fs;
#[allow(dead_code)]
pub(crate) mod attribution;
pub mod audit_trail;
pub(crate) mod binary_detect;
#[allow(dead_code)]
pub(crate) mod bounce_tracker;
pub(crate) mod budget;
#[allow(dead_code)]
pub(crate) mod budget_tracker;
#[allow(dead_code)]
pub(crate) mod budgets;
pub mod cache;
#[allow(dead_code)]
pub(crate) mod cache_diagnostics;
pub(crate) mod cache_telemetry;
pub mod capabilities;
pub mod capsule_transport;
#[allow(dead_code)]
pub(crate) mod chain_compression;
pub(crate) mod cli_cache;
pub(crate) mod client_capabilities;
pub(crate) mod client_constraints;
pub(crate) mod cloud_files;
pub mod config;
pub(crate) mod config_heal;
pub mod consolidation;
#[allow(dead_code)]
pub(crate) mod consolidation_engine;
#[allow(dead_code)]
pub(crate) mod content_handle;
pub mod context_capsule;
pub(crate) mod contextops;
pub(crate) mod conversation;
pub mod crash_log;
pub(crate) mod data_consolidate;
pub mod data_dir;
pub(crate) mod debug_log;
#[allow(unused)]
pub(crate) mod delivered_ranges;
#[allow(dead_code)]
pub(crate) mod delta_response;
#[allow(dead_code)]
pub(crate) mod diagnostics_store;
#[allow(dead_code)]
pub(crate) mod editor_signal;
pub(crate) mod egress;
#[allow(dead_code)]
pub(crate) mod error;
pub mod events;
pub(crate) mod eviction_orchestrator;
pub(crate) mod evidence;
#[allow(dead_code)]
pub(crate) mod evidence_classification;
#[allow(dead_code)]
pub(crate) mod evidence_ledger;
pub mod extension_registry;
#[allow(dead_code)]
pub(crate) mod extractors;
pub mod feedback;
pub(crate) mod fep_prefetch;
pub(crate) mod filters;
pub mod free_energy_budget;
#[allow(dead_code)]
pub(crate) mod gain;
pub(crate) mod git;
#[allow(dead_code)]
pub(crate) mod git_cache;
pub(crate) mod git_signals;
pub(crate) mod git_util;
pub(crate) mod godot;
pub mod gotcha_tracker;
#[allow(dead_code)]
pub(crate) mod handle;
pub mod hasher;
#[allow(dead_code)]
pub(crate) mod heatmap;
pub mod hebbian_cache;
pub mod hnsw;
#[allow(dead_code)]
pub(crate) mod home;
pub mod homeostasis;
pub(crate) mod immune_detector;
#[allow(dead_code)]
pub(crate) mod marginal_gate;
pub mod mcp_catalog;
#[allow(dead_code)]
pub(crate) mod negative_knowledge;
pub mod ocla;
#[allow(dead_code)]
pub(crate) mod ocla_bus;
pub(crate) mod quality_benchmark;
pub(crate) mod qubo_select;
#[allow(dead_code)]
pub(crate) mod query_aware;
#[allow(dead_code)]
pub(crate) mod session_budget;
#[allow(dead_code)]
pub(crate) mod work_graph;

pub(crate) mod agent_registry;
pub mod compliance;
pub mod compliance_report;
pub(crate) mod edit_metering;
pub(crate) mod edit_quality;
pub(crate) mod efficacy;
pub mod evidence_bundle;
#[allow(dead_code)]
pub(crate) mod grammar_usage;
pub(crate) mod graph_cache;
pub(crate) mod http_client;
#[allow(dead_code)]
pub(crate) mod ide_permissions;
#[allow(dead_code)]
pub(crate) mod import_resolver;
#[allow(dead_code)]
pub(crate) mod index_admission;
#[allow(dead_code)]
pub(crate) mod index_bundle;
pub(crate) mod index_filter;
pub(crate) mod index_namespace;
pub mod index_orchestrator;
pub(crate) mod index_paths;
#[allow(dead_code)]
pub(crate) mod index_progress;
#[allow(dead_code)]
pub(crate) mod ingestion;
pub mod input_filters;
pub(crate) mod installation_id;
pub(crate) mod instruction_compiler;
pub(crate) mod integrity;
#[allow(dead_code)]
pub(crate) mod intent_engine;
pub(crate) mod intent_lang;
pub mod intent_protocol;
pub(crate) mod intent_router;
#[allow(dead_code)]
pub(crate) mod introspect;
#[allow(dead_code)]
pub(crate) mod io_boundary;
pub mod io_health;
#[allow(dead_code)]
pub(crate) mod journal;
pub mod jsonc;
pub(crate) mod knowledge_vault;
#[allow(dead_code)]
pub(crate) mod language_capabilities;
#[cfg(target_os = "macos")]
pub(crate) mod launchd;
pub(crate) mod layout_pin;
pub(crate) mod learning_sync;
pub(crate) mod levenshtein;
pub(crate) mod limits;
#[allow(dead_code)]
pub(crate) mod llm_enhance;
pub(crate) mod llm_feedback;
pub mod locomo;
pub(crate) mod logging;
pub mod mcp_manifest;
#[allow(dead_code, unreachable_pub)]
pub mod mdl_mode;
#[allow(dead_code)]
pub(crate) mod mdl_selector;
#[allow(dead_code)]
pub(crate) mod multi_repo;
pub(crate) mod nc_compress;
pub mod ocp;
pub mod openapi;
pub(crate) mod output_echo;
pub(crate) mod owasp_alignment;
pub(crate) mod path_locks;
pub(crate) mod path_mode_memory;
pub mod path_resolve;
pub mod paths;
pub mod pathutil;
#[allow(dead_code)]
pub(crate) mod persona;
#[allow(dead_code)]
pub(crate) mod pipeline;
pub mod plugins;
pub(crate) mod portable_binary;
pub(crate) mod profile_suggest;
pub(crate) mod profiles;
pub(crate) mod project_hash;
pub mod protocol;
pub mod provider_bandit;
#[allow(dead_code)]
pub(crate) mod provider_cache;
pub mod providers;
pub(crate) mod read_stub_index;
pub(crate) mod recovery;
pub(crate) mod redaction;
pub mod reference_docs;
#[allow(dead_code)]
pub(crate) mod roles;
pub(crate) mod route_extractor;
pub mod saliency;
#[allow(dead_code)]
pub(crate) mod sandbox;
#[cfg(target_os = "linux")]
pub(crate) mod sandbox_landlock;
pub(crate) mod sandbox_seatbelt;
pub(crate) mod sanitize;
pub(crate) mod savings_autopush;
#[allow(dead_code)]
pub(crate) mod savings_footer;
pub mod savings_ledger;
#[allow(dead_code)]
pub(crate) mod scent_field;
pub(crate) mod search_delta;
pub mod search_index;
#[allow(dead_code)]
pub(crate) mod secret_detection;
#[allow(dead_code)]
pub(crate) mod security_posture;
pub mod sensitivity;
pub mod server_capabilities;
pub mod session_token;
pub(crate) mod share;
pub mod shell_allowlist;
pub mod startup_guard;
pub mod stats;
#[allow(dead_code)]
pub(crate) mod structural_diff;
pub mod symbol_map;
pub(crate) mod syntax_validate;
pub(crate) mod task_benchmark;
#[allow(dead_code)]
pub(crate) mod task_briefing;
/// macOS Seatbelt self-sandbox (#356): wraps launchd-owned daemon/proxy/updater
/// in a `sandbox-exec` profile that denies `~/Documents`/`~/Desktop`/
/// `~/Downloads`, so the TCC privacy prompt can never appear.
#[cfg(target_os = "macos")]
pub mod tcc_guard_sandbox;
pub mod tdd_schema;
pub mod telemetry;
pub(crate) mod telemetry_ledger;
pub mod terse;
#[allow(dead_code)]
pub(crate) mod theme;
pub(crate) mod threshold_learning;
#[allow(dead_code)]
pub(crate) mod tokenizer_translation_driver;
pub mod tokens;
pub(crate) mod tool_health;
pub(crate) mod tool_lifecycle;
pub mod tool_profiles;
pub(crate) mod transcript_compact;
pub(crate) mod update_scheduler;
pub(crate) mod updater;
pub(crate) mod version_check;
pub(crate) mod visualizer;
pub(crate) mod walk_filter;
/// WASM extension runtime (`wasm-abi-v1`): sandboxed, language-independent
/// compressors and providers. Feature-gated behind `wasm`.
#[cfg(feature = "wasm")]
pub(crate) mod wasm_ext;
pub mod web;
pub mod workflow;
pub(crate) mod workspace_config;
#[allow(dead_code)]
pub(crate) mod wrapped;
pub(crate) mod wrapped_share;
pub(crate) mod wrapped_svg;
pub(crate) mod xdg_migrate;

// ---------------------------------------------------------------------------
// Feature-gated modules
// ---------------------------------------------------------------------------
pub mod archive;
#[allow(dead_code)]
pub(crate) mod archive_fts;
pub(crate) mod artifact_index;
pub(crate) mod artifacts;
pub(crate) mod ast_walk;
pub(crate) mod buddy;
#[cfg(feature = "tree-sitter")]
pub(crate) mod chunks_ts;
pub(crate) mod context_gc;
pub(crate) mod deep_queries;
pub(crate) mod deps;
pub mod editor_registry;
pub(crate) mod firewall;
pub(crate) mod memory_branch;
pub mod pathjail;
pub(crate) mod relevance_gate;
pub mod signatures;
#[cfg(feature = "tree-sitter")]
pub(crate) mod signatures_ts;
#[allow(dead_code)]
pub(crate) mod storage_maintenance;
pub(crate) mod structured_compact;
pub(crate) mod type_ref_edges;
pub(crate) mod workspace_trust;
