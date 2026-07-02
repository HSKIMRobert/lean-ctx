//! Signed usage-evidence export (enterprise#36, EU-AI-Act evidence trail).
//!
//! An [`EvidenceExportV1`] is a self-verifying JSON artifact over a time
//! window of `usage_events`: daily aggregates (day × person × project ×
//! model), the window totals, a BLAKE3 digest of the canonical row bytes and
//! an Ed25519 signature with the gateway's persistent machine identity — the
//! same keystore the signed savings ledger uses (enterprise#19), so one public
//! key verifies both artifact families.
//!
//! Verification is offline (`verify`): recompute canonical bytes, check
//! digest, check signature. Any altered byte fails. The artifact is a
//! deterministic function of the database contents and the window — exporting
//! twice yields byte-identical rows (stable ORDER BY, rounded sums).

use ed25519_dalek::Signer;
use serde::{Deserialize, Serialize};

/// Schema discriminator for [`EvidenceExportV1`].
pub const EVIDENCE_SCHEMA_V1: &str = "leanctx.evidence.v1";

/// Signed, exportable usage evidence over a window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceExportV1 {
    /// Discriminator so a verifier can refuse unrelated signed JSON.
    pub schema: String,
    /// Window bounds (inclusive), RFC 3339 UTC.
    pub from: String,
    pub to: String,
    /// Daily aggregates: day × person × project × model × provider.
    pub rows: Vec<serde_json::Value>,
    /// Row count (redundant with `rows.len()`, part of the signed payload).
    pub row_count: u64,
    /// Window totals over the raw events (not the aggregates).
    pub totals: EvidenceTotals,
    /// BLAKE3 hex digest of the canonical `rows` bytes.
    pub rows_digest_blake3: String,
    /// Ed25519 public key (hex). `None` until signed.
    pub signer_public_key: Option<String>,
    /// Ed25519 signature over the canonical bytes (hex). `None` until signed.
    pub signature: Option<String>,
}

/// Aggregate totals of the exported window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceTotals {
    pub requests: u64,
    pub cost_usd: f64,
    pub saved_usd: f64,
    pub reference_cost_usd: f64,
    pub persons: u64,
}

/// Outcome of [`EvidenceExportV1::verify`].
#[derive(Debug, Clone)]
pub struct EvidenceVerifyResult {
    pub signature_valid: bool,
    pub digest_valid: bool,
    pub signer_public_key: Option<String>,
    pub error: Option<String>,
}

impl EvidenceExportV1 {
    /// Builds the unsigned artifact from already-aggregated rows + totals.
    #[must_use]
    pub fn build(
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
        rows: Vec<serde_json::Value>,
        totals: EvidenceTotals,
    ) -> Self {
        let digest = rows_digest(&rows);
        Self {
            schema: EVIDENCE_SCHEMA_V1.to_string(),
            from: from.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            to: to.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            row_count: rows.len() as u64,
            rows,
            totals,
            rows_digest_blake3: digest,
            signer_public_key: None,
            signature: None,
        }
    }

    /// Deterministic bytes that get signed/verified: the whole struct with the
    /// two signature fields cleared (same convention as `SignedSavingsBatchV1`).
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        let mut clone = self.clone();
        clone.signature = None;
        clone.signer_public_key = None;
        serde_json::to_vec(&clone).map_err(|e| format!("serialize for signing: {e}"))
    }

    /// Signs with the persistent machine identity (`agent_identity` keystore).
    pub fn sign(&mut self, agent_id: &str) -> Result<(), String> {
        let key = crate::core::agent_identity::get_or_create_keypair(agent_id)?;
        self.sign_with_key(&key)
    }

    /// Signs with an explicit key (hermetic tests).
    pub fn sign_with_key(&mut self, key: &ed25519_dalek::SigningKey) -> Result<(), String> {
        self.signature = None;
        self.signer_public_key = None;
        let canonical = self.canonical_bytes()?;
        let sig = key.sign(&canonical);
        self.signer_public_key = Some(crate::core::agent_identity::hex_encode(
            &key.verifying_key().to_bytes(),
        ));
        self.signature = Some(crate::core::agent_identity::hex_encode(&sig.to_bytes()));
        Ok(())
    }

    /// Offline verification: digest over `rows`, then the Ed25519 signature
    /// over the canonical bytes with the embedded public key.
    #[must_use]
    pub fn verify(&self) -> EvidenceVerifyResult {
        let fail = |digest_valid: bool, msg: &str| EvidenceVerifyResult {
            signature_valid: false,
            digest_valid,
            signer_public_key: self.signer_public_key.clone(),
            error: Some(msg.to_string()),
        };

        if self.schema != EVIDENCE_SCHEMA_V1 {
            return fail(false, "unknown schema");
        }
        let digest_valid = rows_digest(&self.rows) == self.rows_digest_blake3
            && self.row_count == self.rows.len() as u64;
        if !digest_valid {
            return fail(false, "rows digest mismatch — rows were altered");
        }

        let (Some(sig_hex), Some(pk_hex)) = (&self.signature, &self.signer_public_key) else {
            return fail(digest_valid, "artifact is not signed");
        };
        let Ok(pk_bytes) = crate::core::agent_identity::hex_decode(pk_hex) else {
            return fail(digest_valid, "invalid public key hex");
        };
        let Ok(pk_arr) = <[u8; 32]>::try_from(pk_bytes.as_slice()) else {
            return fail(digest_valid, "public key must be 32 bytes");
        };
        let Ok(vk) = ed25519_dalek::VerifyingKey::from_bytes(&pk_arr) else {
            return fail(digest_valid, "invalid Ed25519 public key");
        };
        let Ok(sig_bytes) = crate::core::agent_identity::hex_decode(sig_hex) else {
            return fail(digest_valid, "invalid signature hex");
        };
        let Ok(sig_arr) = <[u8; 64]>::try_from(sig_bytes.as_slice()) else {
            return fail(digest_valid, "signature must be 64 bytes");
        };
        let signature = ed25519_dalek::Signature::from_bytes(&sig_arr);
        let canonical = match self.canonical_bytes() {
            Ok(b) => b,
            Err(e) => return fail(digest_valid, &e),
        };
        use ed25519_dalek::Verifier;
        match vk.verify(&canonical, &signature) {
            Ok(()) => EvidenceVerifyResult {
                signature_valid: true,
                digest_valid,
                signer_public_key: self.signer_public_key.clone(),
                error: None,
            },
            Err(_) => fail(digest_valid, "signature does not match canonical payload"),
        }
    }
}

/// BLAKE3 hex over the concatenated canonical row bytes (order-sensitive —
/// the SQL ORDER BY is part of the contract).
fn rows_digest(rows: &[serde_json::Value]) -> String {
    let mut hasher = blake3::Hasher::new();
    for row in rows {
        if let Ok(bytes) = serde_json::to_vec(row) {
            hasher.update(&bytes);
            hasher.update(b"\n");
        }
    }
    hasher.finalize().to_hex().to_string()
}

/// Builds + signs the artifact from the store for `[from, to]`.
pub async fn generate(
    pool: &deadpool_postgres::Pool,
    from: chrono::DateTime<chrono::Utc>,
    to: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<EvidenceExportV1> {
    let rows = super::store::evidence_rows(pool, from, to).await?;
    let totals = window_totals(pool, from, to).await?;
    let mut artifact = EvidenceExportV1::build(from, to, rows, totals);
    artifact
        .sign("gateway-evidence")
        .map_err(|e| anyhow::anyhow!("sign evidence export: {e}"))?;
    Ok(artifact)
}

async fn window_totals(
    pool: &deadpool_postgres::Pool,
    from: chrono::DateTime<chrono::Utc>,
    to: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<EvidenceTotals> {
    let client = pool.get().await?;
    let row = client
        .query_one(
            "SELECT count(*), coalesce(sum(cost_usd), 0), coalesce(sum(saved_usd), 0), \
                    coalesce(sum(reference_cost_usd), 0), count(DISTINCT person) \
             FROM usage_events WHERE ts >= $1 AND ts <= $2",
            &[&from, &to],
        )
        .await?;
    Ok(EvidenceTotals {
        requests: u64::try_from(row.get::<_, i64>(0)).unwrap_or(0),
        cost_usd: row.get::<_, f64>(1),
        saved_usd: row.get::<_, f64>(2),
        reference_cost_usd: row.get::<_, f64>(3),
        persons: u64::try_from(row.get::<_, i64>(4)).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> EvidenceExportV1 {
        let from = chrono::DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let to = chrono::DateTime::parse_from_rfc3339("2026-06-30T23:59:59Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        EvidenceExportV1::build(
            from,
            to,
            vec![
                json!({"date":"2026-06-01","person":"p:abc","project":"web","model":"claude-sonnet-4-5","provider":"Anthropic","requests":12,"cost_usd":1.25}),
                json!({"date":"2026-06-02","person":"p:abc","project":"web","model":"gpt-4o-mini","provider":"foundry","requests":30,"cost_usd":0.42}),
            ],
            EvidenceTotals {
                requests: 42,
                cost_usd: 1.67,
                saved_usd: 0.9,
                reference_cost_usd: 3.1,
                persons: 1,
            },
        )
    }

    fn test_key() -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[7u8; 32])
    }

    #[test]
    fn sign_then_verify_roundtrips() {
        let mut artifact = sample();
        artifact.sign_with_key(&test_key()).unwrap();
        let result = artifact.verify();
        assert!(result.digest_valid);
        assert!(result.signature_valid, "{:?}", result.error);
        assert_eq!(
            result.signer_public_key, artifact.signer_public_key,
            "verify must report the embedded signer"
        );
    }

    #[test]
    fn any_tamper_breaks_verification() {
        let mut artifact = sample();
        artifact.sign_with_key(&test_key()).unwrap();

        // Row tampering breaks the digest.
        let mut tampered = artifact.clone();
        tampered.rows[0]["cost_usd"] = json!(0.01);
        let r = tampered.verify();
        assert!(!r.digest_valid && !r.signature_valid);

        // Totals tampering keeps the digest but breaks the signature.
        let mut tampered = artifact.clone();
        tampered.totals.cost_usd = 0.0;
        let r = tampered.verify();
        assert!(r.digest_valid);
        assert!(!r.signature_valid);

        // Unsigned artifacts are refused.
        let unsigned = sample();
        assert!(!unsigned.verify().signature_valid);
    }

    #[test]
    fn export_is_deterministic_for_same_rows() {
        // #498-adjacent: the artifact is a pure function of (window, rows).
        let a = sample();
        let b = sample();
        assert_eq!(a.rows_digest_blake3, b.rows_digest_blake3);
        assert_eq!(
            a.canonical_bytes().unwrap(),
            b.canonical_bytes().unwrap(),
            "same inputs must produce byte-identical canonical payloads"
        );
    }
}
