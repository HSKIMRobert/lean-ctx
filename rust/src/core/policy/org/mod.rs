//! Org-level policy distribution (ADR-023, GL #674).
//!
//! An organisation distributes a centrally signed [`OrgPolicyV1`] artifact.
//! When installed, signature-verified, and the signer's key is trust-pinned,
//! the runtime folds it in as an un-bypassable enforcement floor beneath the
//! local project pack.

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

/// Load + verify + trust-check the installed org policy artifact.
/// Returns `Some(resolved)` only when all three hold:
/// 1. An artifact is installed on disk
/// 2. Its Ed25519 signature is valid
/// 3. The signer's public key is trust-pinned
/// 4. The admin declared `enforced = true`
#[must_use]
pub fn active_resolved() -> Option<ResolvedPolicy> {
    let artifact = store::load_active()?;

    let result = artifact.verify();
    if !result.signature_valid {
        tracing::warn!(
            "org policy: signature invalid ({}), ignoring",
            result.error.as_deref().unwrap_or("unknown")
        );
        return None;
    }

    let signer = artifact.signer_public_key.as_deref()?;
    if !trust::is_trusted(signer) {
        tracing::debug!("org policy: signer not trust-pinned, ignoring (fail-open)");
        return None;
    }

    if !artifact.enforced {
        tracing::debug!("org policy: admin did not set enforced=true, skipping");
        return None;
    }

    match artifact.resolved() {
        Ok(resolved) => Some(resolved),
        Err(e) => {
            tracing::warn!("org policy: failed to resolve pack: {e}");
            None
        }
    }
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
    let Some(artifact) = store::load_active() else {
        return OrgStatus {
            pinned_anchors: trust::trusted_keys().len(),
            ..Default::default()
        };
    };
    let verify = artifact.verify();
    let signer_key = artifact.signer_public_key.clone();
    let trusted = signer_key.as_deref().is_some_and(trust::is_trusted);
    let resolve_result = artifact.resolved();

    OrgStatus {
        present: true,
        source: store::source_path(),
        org: Some(artifact.org.clone()),
        policy_version: Some(artifact.policy_version.clone()),
        enforced: artifact.enforced,
        issued_at: Some(artifact.issued_at.clone()),
        signer_public_key: signer_key,
        signature_valid: verify.signature_valid,
        trusted,
        applied: trusted && verify.signature_valid && artifact.enforced && resolve_result.is_ok(),
        resolve_error: resolve_result.err().map(|e| e.to_string()),
        pinned_anchors: trust::trusted_keys().len(),
    }
}
