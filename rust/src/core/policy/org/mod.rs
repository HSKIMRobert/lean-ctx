//! Org-level policy distribution — OSS stub (ADR-023).
//!
//! The full implementation (signed artifact verification, trust pinning,
//! policy resolution) lives in `lean-ctx-enterprise/commercial-core`.
//! This stub preserves the public API so OSS code compiles.
//! `active_resolved()` always returns `None` — local policy only.

pub mod model;
pub mod store;
pub mod trust;

use std::path::PathBuf;

pub use model::{OrgPolicyV1, OrgVerifyResult};
pub use trust::{TrustStore, TrustedKey};

use crate::core::policy::ResolvedPolicy;

#[must_use]
pub fn org_key_id(org: &str) -> String {
    let safe: String = org
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    format!("org-{safe}")
}

#[must_use]
pub fn active_resolved() -> Option<ResolvedPolicy> {
    None
}

#[derive(Debug, Clone, Default)]
pub struct OrgStatus {
    pub present: bool,
    pub source: Option<PathBuf>,
    pub org: Option<String>,
    pub policy_version: Option<String>,
    pub enforced: bool,
    pub issued_at: Option<String>,
    pub signer_public_key: Option<String>,
    pub signature_valid: bool,
    pub trusted: bool,
    pub applied: bool,
    pub resolve_error: Option<String>,
    pub pinned_anchors: usize,
}

#[must_use]
pub fn status() -> OrgStatus {
    OrgStatus::default()
}
