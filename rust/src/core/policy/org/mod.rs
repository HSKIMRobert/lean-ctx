//! Central, signed org policy distribution (GL #674).
//!
//! An organisation authors one policy pack, **signs** it into an
//! [`OrgPolicyV1`] artifact ([`model`]) with its org key, and distributes both
//! the artifact and its **public key**. Each endpoint *pins* that key once,
//! out-of-band ([`trust`]), then installs artifacts ([`store`]); from then on
//! the runtime folds the org pack in as an un-bypassable **floor**
//! ([`crate::core::policy::floor`]) beneath the local project pack.
//!
//! Two independent checks gate application, both required:
//! 1. **signature** — the artifact's bytes were signed by the embedded key
//!    ([`OrgPolicyV1::verify`]); and
//! 2. **trust** — that key is one this endpoint pinned ([`trust::is_trusted`]).
//!
//! With no key pinned, a present artifact is informational only (opt-in). An
//! invalid or untrusted artifact is never applied and never bricks the agent
//! (fail-open) — it is logged and surfaced by `policy org status`.

pub mod model;
pub mod store;
pub mod trust;

use std::path::PathBuf;

pub use model::{OrgPolicyV1, OrgVerifyResult};
pub use trust::{TrustStore, TrustedKey};

use crate::core::policy::ResolvedPolicy;

/// Keystore identity used for an org's signing key. Namespaced + sanitised so
/// `org sign --org "ACME Corp"` maps to a safe, stable key file.
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

/// The org policy resolved to a [`ResolvedPolicy`] **iff** it is present,
/// signed by a trusted key, and resolves cleanly. Returns `None` (after a
/// `warn!`) in every other case so the caller falls back to the local pack
/// alone. This is the single entry point the runtime uses.
#[must_use]
pub fn active_resolved() -> Option<ResolvedPolicy> {
    let artifact = store::load_active()?;

    let verdict = artifact.verify();
    if !verdict.signature_valid {
        tracing::warn!(
            "org policy: signature invalid ({}); not applied",
            verdict.error.as_deref().unwrap_or("unknown")
        );
        return None;
    }

    let signer = verdict.signer_public_key.as_deref().unwrap_or_default();
    if !trust::is_trusted(signer) {
        tracing::warn!(
            "org policy: signer key not pinned; not applied \
             (pin it with `lean-ctx policy org trust <key>`)"
        );
        return None;
    }

    // Advisory rollout: a signed, trusted artifact with `enforced = false` is
    // distributed for preview (`policy org show`) but is deliberately NOT folded
    // into enforcement, so admins can stage a policy before turning it on.
    if !artifact.enforced {
        return None;
    }

    match artifact.resolved() {
        Ok(resolved) => Some(resolved),
        Err(e) => {
            tracing::warn!("org policy: pack does not resolve ({e}); not applied");
            None
        }
    }
}

/// A snapshot of org-policy state for `policy org status` / `show`.
#[derive(Debug, Clone, Default)]
pub struct OrgStatus {
    /// Whether an artifact is present via the source chain at all.
    pub present: bool,
    pub source: Option<PathBuf>,
    pub org: Option<String>,
    pub policy_version: Option<String>,
    pub enforced: bool,
    pub issued_at: Option<String>,
    pub signer_public_key: Option<String>,
    /// Cryptographic signature check (independent of trust).
    pub signature_valid: bool,
    /// Whether the signer key is pinned on this endpoint.
    pub trusted: bool,
    /// `true` ⇔ the policy is actually enforced (present ∧ signed ∧ trusted ∧
    /// resolves). This is the bottom line for an auditor.
    pub applied: bool,
    /// Set when the pack body fails to resolve despite a valid signature.
    pub resolve_error: Option<String>,
    /// Count of pinned trust anchors on this endpoint.
    pub pinned_anchors: usize,
}

/// Compute the current org-policy [`OrgStatus`] for display.
#[must_use]
pub fn status() -> OrgStatus {
    let mut s = OrgStatus {
        pinned_anchors: trust::trusted_keys().len(),
        ..OrgStatus::default()
    };
    let Some(source) = store::source_path() else {
        return s;
    };
    s.present = true;
    s.source = Some(source.clone());
    let Ok(artifact) = store::read(&source) else {
        return s;
    };
    s.org = Some(artifact.org.clone());
    s.policy_version = Some(artifact.policy_version.clone());
    s.enforced = artifact.enforced;
    s.issued_at = Some(artifact.issued_at.clone());

    let verdict = artifact.verify();
    s.signature_valid = verdict.signature_valid;
    s.signer_public_key.clone_from(&verdict.signer_public_key);
    s.trusted = verdict
        .signer_public_key
        .as_deref()
        .is_some_and(trust::is_trusted);

    if s.signature_valid && s.trusted {
        match artifact.resolved() {
            // Advisory (`enforced = false`) artifacts are valid + trusted but
            // intentionally not enforced — see `active_resolved`.
            Ok(_) => s.applied = artifact.enforced,
            Err(e) => s.resolve_error = Some(e.to_string()),
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data_dir::isolated_data_dir;

    const PACK: &str = r#"
name = "acme-floor"
version = "1.0.0"
description = "ACME org floor"

[context]
deny_tools = ["ctx_url_read"]
"#;

    fn install_signed(enforced: bool) -> String {
        let key = crate::core::agent_identity::get_or_create_keypair(&org_key_id("acme")).unwrap();
        let mut a = OrgPolicyV1::build("acme", "2026.06.1", enforced, PACK).unwrap();
        a.sign_with_key(&key);
        store::install(&a).unwrap();
        crate::core::agent_identity::hex_encode(&key.verifying_key().to_bytes())
    }

    #[test]
    fn org_key_id_is_sanitised() {
        assert_eq!(org_key_id("ACME Corp"), "org-acme-corp");
        assert_eq!(org_key_id("acme"), "org-acme");
    }

    #[test]
    fn not_applied_without_trust_even_if_signed() {
        let _iso = isolated_data_dir();
        install_signed(true);
        // Signed, present — but the key is not pinned → not applied.
        assert!(active_resolved().is_none());
        let s = status();
        assert!(s.present);
        assert!(s.signature_valid);
        assert!(!s.trusted);
        assert!(!s.applied);
    }

    #[test]
    fn applied_once_signer_is_pinned() {
        let _iso = isolated_data_dir();
        let pk = install_signed(true);
        trust::pin("acme", &pk).unwrap();
        let resolved = active_resolved().expect("trusted + signed → applied");
        assert!(resolved.deny_tools.contains(&"ctx_url_read".to_string()));
        let s = status();
        assert!(s.applied);
        assert!(s.trusted);
        assert_eq!(s.org.as_deref(), Some("acme"));
    }

    #[test]
    fn advisory_artifact_is_trusted_but_not_enforced() {
        let _iso = isolated_data_dir();
        let pk = install_signed(false); // enforced = false
        trust::pin("acme", &pk).unwrap();
        assert!(
            active_resolved().is_none(),
            "advisory policy must not be enforced"
        );
        let s = status();
        assert!(s.present && s.signature_valid && s.trusted);
        assert!(!s.enforced);
        assert!(!s.applied, "advisory ⇒ not applied");
    }

    #[test]
    fn tampered_artifact_is_not_applied() {
        let _iso = isolated_data_dir();
        let pk = install_signed(true);
        trust::pin("acme", &pk).unwrap();
        // Tamper with the installed file on disk.
        let path = store::installed_path().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replace("ctx_url_read", "ctx_read")).unwrap();
        assert!(active_resolved().is_none(), "tampered body must not apply");
        let s = status();
        assert!(!s.signature_valid);
        assert!(!s.applied);
    }
}
