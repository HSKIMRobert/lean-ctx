//! Self-hosted Enterprise license — offline entitlement validation (GL #667).
//!
//! Outcome-fee pilots target data-residency customers (banks, UN agencies) that
//! **self-host** the team server and cannot reach the hosted control plane. Their
//! commercial entitlements (`sso_oidc`, `sso_scim`, `audit_retention`) normally
//! resolve through that control plane ([`crate::cloud_client::refresh_effective_plan`]),
//! so an air-gapped install can never unlock Enterprise governance.
//!
//! A [`LicenseV1`] closes that gap: a **vendor-signed, offline-verifiable** file
//! that elevates the effective plan without a network round-trip. It is:
//! - **Trusted** — only licenses signed by a [`trusted_keys`] vendor key count,
//!   so a customer cannot mint their own Enterprise plan.
//! - **Time-bounded** — an `expires_at` plus a generous offline [`LICENSE_GRACE_DAYS`]
//!   window, so a clock skew or a late renewal never hard-cuts a paying customer.
//! - **Local-Free** — it only ever *unlocks commercial/hosted* entitlements;
//!   neither its absence nor its expiry can disable any local capability. The
//!   conformance test `tests/local_free_invariant.rs` guards this.
//!
//! ## Module map
//! - [`model`] — the [`LicenseV1`] artifact + Ed25519 sign/verify + expiry math.
//! - [`store`] — locate / read / install / uninstall the artifact (pluggable
//!   source: `LEAN_CTX_LICENSE` env path → `<config_dir>/license.json`).
//! - this file — trusted vendor keys, [`active`]/[`effective_plan`] resolution
//!   with grace, and the [`status`] snapshot the CLI renders.

pub mod model;
pub mod store;

pub use model::{LicenseV1, LicenseVerifyResult};

use crate::core::billing::Plan;

/// Days a license keeps granting its entitlements **after** `expires_at`. Mirrors
/// the cloud plan's offline grace ([`crate::cloud_client::PLAN_GRACE_DAYS`]): a
/// network blip never demotes a cloud user, and a late renewal never hard-cuts a
/// self-host customer. Local features are unaffected either way.
pub const LICENSE_GRACE_DAYS: i64 = 30;

/// Additional trusted vendor public keys (hex), comma/space separated. Lets a
/// vendor run their **own** signing key (or a test harness inject an ephemeral
/// one) without recompiling. Production trust is the union of this and the
/// compiled-in [`VENDOR_PUBLIC_KEYS`].
const VENDOR_PUBKEY_ENV: &str = "LEAN_CTX_LICENSE_PUBKEY";

/// Compiled-in trusted vendor signing keys (Ed25519 public keys, hex).
///
/// This is the trust anchor for offline license validation: a license is only
/// honoured when signed by one of these keys, so the grant cannot be forged
/// without the vendor's private key. lean-ctx is open-core, so this is
/// *contractual* enforcement (a determined operator can recompile) — not DRM;
/// the point is a clean, auditable, offline proof of entitlement.
const VENDOR_PUBLIC_KEYS: &[&str] =
    &["7a7f4ca9bb6a080f26b89bcc7e36d9be0b7346661efba853275880f23aa58aaa"];

/// The set of trusted vendor public keys for this process: the compiled-in
/// anchors plus any supplied via the `LEAN_CTX_LICENSE_PUBKEY` env var.
#[must_use]
pub fn trusted_keys() -> Vec<String> {
    let mut keys: Vec<String> = VENDOR_PUBLIC_KEYS
        .iter()
        .map(|k| (*k).to_string())
        .collect();
    if let Ok(extra) = std::env::var(VENDOR_PUBKEY_ENV) {
        for k in extra.split([',', ' ', '\n', '\t']) {
            let k = k.trim();
            if !k.is_empty() {
                keys.push(k.to_string());
            }
        }
    }
    keys
}

/// The currently-active license, if one is installed, verifies against a trusted
/// vendor key, and is within its validity window (expiry + grace). Returns
/// `None` — silently, never erroring — when no license applies, so the caller
/// simply falls back to the cloud/cached plan. Invalid or expired-beyond-grace
/// artifacts are logged and ignored.
#[must_use]
pub fn active() -> Option<LicenseV1> {
    let lic = store::load_active()?;
    let res = lic.verify_against(&trusted_keys());
    if !res.ok() {
        tracing::warn!(
            "license: ignored — {}",
            res.error.as_deref().unwrap_or("invalid")
        );
        return None;
    }
    let now = chrono::Utc::now();
    if lic.is_expired(now) {
        let past = lic.days_past_expiry(now);
        if past > LICENSE_GRACE_DAYS {
            tracing::warn!(
                "license: expired {past}d ago (beyond {LICENSE_GRACE_DAYS}d grace) — ignored"
            );
            return None;
        }
        tracing::info!("license: expired {past}d ago, within {LICENSE_GRACE_DAYS}d grace");
    }
    Some(lic)
}

/// The commercial plan granted by the active license, if any. The plan resolver
/// ([`crate::cloud_client`]) elevates the effective plan to the higher of this
/// and the cloud/cached plan.
#[must_use]
pub fn effective_plan() -> Option<Plan> {
    active().map(|l| l.plan)
}

/// A negotiated audit-retention override from the active license, if set.
#[must_use]
pub fn audit_retention_override() -> Option<u32> {
    active().and_then(|l| l.audit_retention_days)
}

/// A snapshot of the license state for `lean-ctx license status` / `verify`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LicenseStatus {
    pub installed: bool,
    pub source_path: Option<String>,
    pub signature_valid: bool,
    pub trusted: bool,
    pub signer_public_key: Option<String>,
    pub customer: Option<String>,
    pub plan: Option<String>,
    pub issued_at: Option<String>,
    pub expires_at: Option<String>,
    pub expired: bool,
    pub within_grace: bool,
    pub days_past_expiry: i64,
    pub audit_retention_days: Option<u32>,
    pub note: Option<String>,
    /// `true` when this license is currently granting entitlements.
    pub active: bool,
    pub error: Option<String>,
}

impl LicenseStatus {
    fn empty() -> Self {
        Self {
            installed: false,
            source_path: None,
            signature_valid: false,
            trusted: false,
            signer_public_key: None,
            customer: None,
            plan: None,
            issued_at: None,
            expires_at: None,
            expired: false,
            within_grace: false,
            days_past_expiry: 0,
            audit_retention_days: None,
            note: None,
            active: false,
            error: None,
        }
    }
}

/// Resolve the full license status for display, including verification, expiry
/// and whether it is currently granting entitlements.
#[must_use]
pub fn status() -> LicenseStatus {
    let Some(path) = store::source_path() else {
        return LicenseStatus::empty();
    };
    let source_path = Some(path.display().to_string());
    let lic = match store::read(&path) {
        Ok(l) => l,
        Err(e) => {
            return LicenseStatus {
                installed: true,
                source_path,
                error: Some(e),
                ..LicenseStatus::empty()
            };
        }
    };
    let res = lic.verify_against(&trusted_keys());
    let now = chrono::Utc::now();
    let expired = lic.is_expired(now);
    let past = lic.days_past_expiry(now);
    let within_grace = expired && past <= LICENSE_GRACE_DAYS;
    let active = res.ok() && (!expired || within_grace);

    LicenseStatus {
        installed: true,
        source_path,
        signature_valid: res.signature_valid,
        trusted: res.trusted,
        signer_public_key: res.signer_public_key,
        customer: Some(lic.customer),
        plan: Some(lic.plan.as_str().to_string()),
        issued_at: Some(lic.issued_at),
        expires_at: lic.expires_at,
        expired,
        within_grace,
        days_past_expiry: if expired { past } else { 0 },
        audit_retention_days: lic.audit_retention_days,
        note: lic.note,
        active,
        error: res.error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data_dir::isolated_data_dir;

    fn key() -> ed25519_dalek::SigningKey {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).unwrap();
        ed25519_dalek::SigningKey::from_bytes(&seed)
    }

    fn pubhex(k: &ed25519_dalek::SigningKey) -> String {
        crate::core::agent_identity::hex_encode(&k.verifying_key().to_bytes())
    }

    fn install_signed(k: &ed25519_dalek::SigningKey, expires_at: Option<String>) {
        let mut lic = LicenseV1::new("acme", Plan::Enterprise, expires_at, None, None);
        lic.sign_with_key(k);
        store::install(&lic).unwrap();
    }

    #[test]
    fn no_license_means_no_plan() {
        let _iso = isolated_data_dir();
        crate::test_env::remove_var(VENDOR_PUBKEY_ENV);
        assert!(effective_plan().is_none());
        assert!(!status().installed);
    }

    #[test]
    fn trusted_license_grants_enterprise() {
        let _iso = isolated_data_dir();
        let k = key();
        crate::test_env::set_var(VENDOR_PUBKEY_ENV, pubhex(&k));
        install_signed(&k, None);
        assert_eq!(effective_plan(), Some(Plan::Enterprise));
        let st = status();
        assert!(st.active && st.trusted && st.signature_valid && !st.expired);
        crate::test_env::remove_var(VENDOR_PUBKEY_ENV);
    }

    #[test]
    fn untrusted_signer_is_ignored() {
        let _iso = isolated_data_dir();
        let k = key();
        crate::test_env::remove_var(VENDOR_PUBKEY_ENV); // do NOT trust k
        install_signed(&k, None);
        assert!(effective_plan().is_none());
        let st = status();
        assert!(st.installed && st.signature_valid && !st.trusted && !st.active);
    }

    #[test]
    fn expired_beyond_grace_is_ignored() {
        let _iso = isolated_data_dir();
        let k = key();
        crate::test_env::set_var(VENDOR_PUBKEY_ENV, pubhex(&k));
        install_signed(&k, Some("2000-01-01T00:00:00Z".to_string()));
        assert!(effective_plan().is_none());
        let st = status();
        assert!(st.installed && st.trusted && st.expired && !st.within_grace && !st.active);
        crate::test_env::remove_var(VENDOR_PUBKEY_ENV);
    }
}
