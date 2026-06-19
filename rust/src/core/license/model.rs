//! The signed self-hosted license artifact (GL #667).
//!
//! [`LicenseV1`] grants commercial **plan entitlements** (typically
//! [`Plan::Enterprise`]) to an air-gapped self-host **without a network
//! round-trip**: the customer installs a vendor-signed file and the team server
//! unlocks Enterprise governance offline. It is the outcome-fee enabler for
//! data-residency customers (banks, UN agencies) that cannot reach the hosted
//! control plane.
//!
//! Signing mirrors [`crate::core::savings_ledger::signed_batch`] and the other
//! signed artifacts: the two signature fields are cleared while computing the
//! canonical bytes, so a verifier reproduces the exact signed payload offline.
//! Unlike the per-machine artifacts, a license is only honoured when its signer
//! is a **trusted vendor key** ([`super::trusted_keys`]) — a customer cannot
//! mint their own Enterprise plan.
//!
//! **Local-Free Invariant:** a license only ever *unlocks commercial/hosted*
//! entitlements. It never gates — and the absence/expiry of a license never
//! disables — any local capability.

use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};

use crate::core::billing::Plan;

pub const SCHEMA_VERSION: u32 = 1;
pub const KIND: &str = "lean-ctx.license";

/// Outcome of verifying a [`LicenseV1`] — offline, no network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseVerifyResult {
    /// The Ed25519 signature is valid for the embedded key.
    pub signature_valid: bool,
    /// The signer is one of the trusted vendor keys.
    pub trusted: bool,
    pub signer_public_key: Option<String>,
    pub error: Option<String>,
}

impl LicenseVerifyResult {
    /// A license is usable only when both the signature verifies *and* the
    /// signer is a trusted vendor key.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.signature_valid && self.trusted
    }
}

/// A signed, offline-verifiable Enterprise license.
///
/// `signature` / `signer_public_key` are excluded from the signed payload (set
/// to `None` while computing the canonical bytes), exactly like the other signed
/// artifacts in the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenseV1 {
    pub schema_version: u32,
    /// Discriminator so a verifier can refuse unrelated signed JSON.
    pub kind: String,
    /// Customer identifier the license is issued to (deal / org reference).
    pub customer: String,
    /// Granted commercial plan (typically [`Plan::Enterprise`]).
    pub plan: Plan,
    /// When the vendor issued the license (RFC 3339).
    pub issued_at: String,
    /// Expiry (RFC 3339). `None` = perpetual.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Optional negotiated override for the audit-retention window (days),
    /// applied on top of the plan's default entitlement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_retention_days: Option<u32>,
    /// Free-form note (e.g. contract reference) — part of the signed payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Ed25519 public key of the signing vendor key (hex). `None` until signed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer_public_key: Option<String>,
    /// Ed25519 signature over the canonical bytes (hex). `None` until signed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl LicenseV1 {
    /// Build an unsigned license.
    #[must_use]
    pub fn new(
        customer: &str,
        plan: Plan,
        expires_at: Option<String>,
        audit_retention_days: Option<u32>,
        note: Option<String>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            kind: KIND.to_string(),
            customer: customer.to_string(),
            plan,
            issued_at: chrono::Utc::now().to_rfc3339(),
            expires_at,
            audit_retention_days,
            note,
            signer_public_key: None,
            signature: None,
        }
    }

    /// Deterministic bytes that get signed/verified: the whole struct with the
    /// two signature fields cleared. Identical on sign and verify.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        let mut clone = self.clone();
        clone.signature = None;
        clone.signer_public_key = None;
        serde_json::to_vec(&clone).map_err(|e| format!("serialize for signing: {e}"))
    }

    /// Sign with an explicit vendor key (the private half stays with the vendor).
    /// The public key is embedded so the artifact is self-describing; trust is
    /// still checked separately against [`super::trusted_keys`].
    pub fn sign_with_key(&mut self, key: &SigningKey) {
        self.signature = None;
        self.signer_public_key = None;
        let canonical = self.canonical_bytes().unwrap_or_default();
        let sig = key.sign(&canonical);
        self.signer_public_key = Some(crate::core::agent_identity::hex_encode(
            &key.verifying_key().to_bytes(),
        ));
        self.signature = Some(crate::core::agent_identity::hex_encode(&sig.to_bytes()));
    }

    /// Verify the embedded signature **and** that the signer is one of `trusted`
    /// vendor keys (hex). Offline, no network. Expiry is a *separate* check
    /// ([`is_expired`](Self::is_expired)) so callers can apply grace.
    #[must_use]
    pub fn verify_against(&self, trusted: &[String]) -> LicenseVerifyResult {
        let fail = |msg: &str| LicenseVerifyResult {
            signature_valid: false,
            trusted: false,
            signer_public_key: self.signer_public_key.clone(),
            error: Some(msg.to_string()),
        };
        if self.kind != KIND {
            return fail("not a license artifact");
        }
        let (Some(sig_hex), Some(pk_hex)) = (&self.signature, &self.signer_public_key) else {
            return fail("license is not signed");
        };
        let (Ok(sig_bytes), Ok(pk_bytes)) = (
            crate::core::agent_identity::hex_decode(sig_hex),
            crate::core::agent_identity::hex_decode(pk_hex),
        ) else {
            return fail("malformed signature or public key hex");
        };
        let canonical = match self.canonical_bytes() {
            Ok(c) => c,
            Err(e) => return fail(&e),
        };
        let signature_valid =
            crate::core::agent_identity::verify_signature(&pk_bytes, &canonical, &sig_bytes);
        if !signature_valid {
            return fail("signature does not match payload (tampered or wrong key)");
        }
        let trusted_ok = trusted.iter().any(|k| k.eq_ignore_ascii_case(pk_hex));
        LicenseVerifyResult {
            signature_valid: true,
            trusted: trusted_ok,
            signer_public_key: Some(pk_hex.clone()),
            error: (!trusted_ok).then(|| "signer is not a trusted vendor key".to_string()),
        }
    }

    /// `true` when the license has an `expires_at` strictly before `now`
    /// (RFC 3339). A malformed timestamp is treated as expired (fail-closed for
    /// the commercial grant; never for local features).
    #[must_use]
    pub fn is_expired(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        match &self.expires_at {
            None => false,
            Some(ts) => match chrono::DateTime::parse_from_rfc3339(ts) {
                Ok(exp) => now > exp.with_timezone(&chrono::Utc),
                Err(_) => true,
            },
        }
    }

    /// Days since expiry (0 if not yet/never expired) — drives the grace window.
    #[must_use]
    pub fn days_past_expiry(&self, now: chrono::DateTime<chrono::Utc>) -> i64 {
        let Some(ts) = &self.expires_at else { return 0 };
        match chrono::DateTime::parse_from_rfc3339(ts) {
            Ok(exp) => ((now - exp.with_timezone(&chrono::Utc)).num_seconds() / 86_400).max(0),
            Err(_) => i64::MAX,
        }
    }

    /// Serialize to the pretty JSON artifact the vendor distributes.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("serialize license: {e}"))
    }

    /// Parse an artifact, rejecting unrelated JSON by `kind`.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let parsed: Self =
            serde_json::from_str(text).map_err(|e| format!("not a valid license artifact: {e}"))?;
        if parsed.kind != KIND {
            return Err(format!(
                "wrong artifact kind '{}' (expected '{KIND}')",
                parsed.kind
            ));
        }
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SigningKey {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).unwrap();
        SigningKey::from_bytes(&seed)
    }

    fn pubhex(k: &SigningKey) -> String {
        crate::core::agent_identity::hex_encode(&k.verifying_key().to_bytes())
    }

    #[test]
    fn sign_then_verify_against_trusted() {
        let k = key();
        let mut lic = LicenseV1::new("acme", Plan::Enterprise, None, None, None);
        lic.sign_with_key(&k);
        let res = lic.verify_against(&[pubhex(&k)]);
        assert!(res.signature_valid && res.trusted && res.ok());
    }

    #[test]
    fn untrusted_signer_is_valid_but_not_trusted() {
        let k = key();
        let mut lic = LicenseV1::new("acme", Plan::Enterprise, None, None, None);
        lic.sign_with_key(&k);
        let res = lic.verify_against(&["00".repeat(32)]); // some other key
        assert!(res.signature_valid);
        assert!(!res.trusted);
        assert!(!res.ok());
    }

    #[test]
    fn tampered_plan_fails_signature() {
        let k = key();
        let mut lic = LicenseV1::new("acme", Plan::Team, None, None, None);
        lic.sign_with_key(&k);
        lic.plan = Plan::Enterprise; // upgrade after signing
        assert!(!lic.verify_against(&[pubhex(&k)]).signature_valid);
    }

    #[test]
    fn expiry_is_evaluated() {
        let past = LicenseV1::new(
            "acme",
            Plan::Enterprise,
            Some("2000-01-01T00:00:00Z".to_string()),
            None,
            None,
        );
        assert!(past.is_expired(chrono::Utc::now()));
        assert!(past.days_past_expiry(chrono::Utc::now()) > 0);

        let perpetual = LicenseV1::new("acme", Plan::Enterprise, None, None, None);
        assert!(!perpetual.is_expired(chrono::Utc::now()));
        assert_eq!(perpetual.days_past_expiry(chrono::Utc::now()), 0);
    }

    #[test]
    fn json_roundtrip_preserves_and_verifies() {
        let k = key();
        let mut lic = LicenseV1::new(
            "acme",
            Plan::Enterprise,
            Some("2030-01-01T00:00:00Z".to_string()),
            Some(2555),
            Some("MoU-2026".to_string()),
        );
        lic.sign_with_key(&k);
        let json = lic.to_json().unwrap();
        let loaded = LicenseV1::from_json(&json).unwrap();
        assert_eq!(loaded, lic);
        assert!(loaded.verify_against(&[pubhex(&k)]).ok());
    }

    #[test]
    fn from_json_rejects_foreign_kind() {
        let json =
            r#"{"schema_version":1,"kind":"x","customer":"a","plan":"enterprise","issued_at":"t"}"#;
        assert!(LicenseV1::from_json(json).is_err());
    }
}
