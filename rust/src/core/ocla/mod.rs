//! Open Context & Token Lifecycle Architecture (OCLA) public OSS contract.
//!
//! Traits and types are defined in the standalone `lean-ctx-ocla` crate and
//! re-exported here so that engine-internal code continues using
//! `crate::core::ocla::*` without import changes.

pub mod budget;
pub mod builtin;
pub mod cache_coordinator;
pub mod cache_delivery;
pub mod cache_tiers;
pub mod cache_types;
pub mod capsule;
pub mod compose_cache;
pub mod content_port;
pub mod grpc_bridge;
pub mod health;
pub mod ledger_export;
pub mod openapi;
pub mod policy_bundle;
pub mod registry;
pub mod regression_gate;
pub mod response_cache;
pub mod routing_experiment;
pub mod routing_quality;
#[cfg(feature = "http-server")]
pub mod runtime;
pub mod shell_cache_allowlist;
pub mod sidecar;
pub mod tracing;
#[allow(dead_code)]
pub mod unified_ledger;
pub mod wire;
#[cfg(feature = "http-server")]
pub mod wire_api;
#[cfg(feature = "http-server")]
pub mod wire_middleware;
pub mod wire_stream;

pub mod traits {
    pub use lean_ctx_ocla::traits::*;
}
pub mod types {
    pub use lean_ctx_ocla::types::*;
}

pub use registry::OclaRegistry;
pub use traits::*;
pub use types::*;
