//! `GET /v1/savings/summary` — the team savings roll-up (the customer-facing
//! "team usage visibility" surface).
//!
//! The savings store holds one append-only JSONL file per signer
//! (`savings_<pubkey>.jsonl`); each line is a [`SignedSavingsBatchV1`] snapshot
//! of that signer's **whole** local ledger (`period = "all"`). Successive batches
//! from the same signer are therefore cumulative re-snapshots, **not** increments
//! — so the honest team total is the sum of each signer's *latest* batch, never
//! the sum of every batch (which would multiply-count). Integrity is enforced at
//! ingest ([`super::savings_ingest`] verifies the Ed25519 signature before
//! storing), so this read path trusts the stored snapshots and parses defensively.
//!
//! Authorisation: gated by [`TeamScope::Audit`](super::team) in the team auth
//! middleware (owner/admin only) — aggregate savings is sensitive team data.

use std::collections::HashMap;
use std::path::Path;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::core::savings_ledger::SignedSavingsBatchV1;

use super::team::TeamAppState;

/// Team-wide savings roll-up, aggregated from each member's latest signed batch.
#[derive(Debug, Default, Serialize)]
pub struct TeamSavingsSummary {
    pub schema_version: u32,
    pub generated_at: String,
    /// Distinct signers (≈ developers/agents) that have reported savings.
    pub member_count: usize,
    pub totals: SavingsTotals,
    /// One row per signer, descending by net saved tokens.
    pub by_member: Vec<MemberSavings>,
    /// Cross-team model breakdown (summed over each member's latest batch).
    pub by_model: Vec<ModelRow>,
}

#[derive(Debug, Default, Serialize)]
pub struct SavingsTotals {
    /// Gross saved tokens (before bounce adjustment).
    pub saved_tokens: u64,
    /// Net saved tokens (gross minus compressed→full re-read bounce).
    pub net_saved_tokens: u64,
    /// Conservative USD upper bound (ignores prompt-cache discounts).
    pub saved_usd: f64,
}

#[derive(Debug, Serialize)]
pub struct MemberSavings {
    /// Truncated signer public key — a stable, privacy-preserving member id.
    pub signer: String,
    pub agent_id: String,
    pub net_saved_tokens: u64,
    pub saved_usd: f64,
    /// `created_at` of the member's most recent batch (RFC 3339).
    pub last_reported: String,
}

#[derive(Debug, Serialize)]
pub struct ModelRow {
    pub model: String,
    pub saved_tokens: u64,
    pub saved_usd: f64,
}

pub async fn v1_savings_summary(State(state): State<TeamAppState>) -> impl IntoResponse {
    let dir = state.team.savings_store_dir.lock().await.clone();
    let summary = tokio::task::spawn_blocking(move || aggregate(&dir))
        .await
        .unwrap_or_default();
    (StatusCode::OK, Json(summary))
}

/// Aggregate the savings store: latest batch per signer, summed across signers.
fn aggregate(dir: &Path) -> TeamSavingsSummary {
    let mut members: Vec<MemberSavings> = Vec::new();
    let mut model_totals: HashMap<String, (u64, f64)> = HashMap::new();
    let mut totals = SavingsTotals::default();

    let Ok(entries) = std::fs::read_dir(dir) else {
        return finalize(totals, members, model_totals);
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let named_savings = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("savings_"));
        let is_jsonl = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("jsonl"));
        if !(named_savings && is_jsonl) {
            continue;
        }
        let Some(batch) = latest_batch(&path) else {
            continue;
        };

        totals.saved_tokens = totals
            .saved_tokens
            .saturating_add(batch.totals.saved_tokens);
        totals.net_saved_tokens = totals
            .net_saved_tokens
            .saturating_add(batch.totals.net_saved_tokens);
        totals.saved_usd += batch.totals.saved_usd;

        for (model, tokens, usd) in &batch.totals.by_model {
            let acc = model_totals.entry(model.clone()).or_default();
            acc.0 = acc.0.saturating_add(*tokens);
            acc.1 += *usd;
        }

        let signer = batch.signer_public_key.as_deref().unwrap_or("unknown");
        members.push(MemberSavings {
            signer: signer.chars().take(16).collect(),
            agent_id: batch.agent_id.clone(),
            net_saved_tokens: batch.totals.net_saved_tokens,
            saved_usd: round_usd(batch.totals.saved_usd),
            last_reported: batch.created_at.clone(),
        });
    }

    finalize(totals, members, model_totals)
}

fn finalize(
    mut totals: SavingsTotals,
    mut members: Vec<MemberSavings>,
    model_totals: HashMap<String, (u64, f64)>,
) -> TeamSavingsSummary {
    totals.saved_usd = round_usd(totals.saved_usd);
    members.sort_by_key(|m| std::cmp::Reverse(m.net_saved_tokens));

    let mut by_model: Vec<ModelRow> = model_totals
        .into_iter()
        .map(|(model, (saved_tokens, usd))| ModelRow {
            model,
            saved_tokens,
            saved_usd: round_usd(usd),
        })
        .collect();
    by_model.sort_by_key(|r| std::cmp::Reverse(r.saved_tokens));
    by_model.truncate(10);

    TeamSavingsSummary {
        schema_version: 1,
        generated_at: chrono::Utc::now().to_rfc3339(),
        member_count: members.len(),
        totals,
        by_member: members,
        by_model,
    }
}

/// The most recent (= last non-empty, parseable) batch in a signer's JSONL file.
fn latest_batch(path: &Path) -> Option<SignedSavingsBatchV1> {
    let content = std::fs::read_to_string(path).ok()?;
    content.lines().rev().find_map(|line| {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        serde_json::from_str::<SignedSavingsBatchV1>(line).ok()
    })
}

fn round_usd(v: f64) -> f64 {
    (v * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::savings_ledger::signed_batch::BatchTotals;

    fn batch(signer: &str, net: u64, usd: f64, created_at: &str) -> SignedSavingsBatchV1 {
        SignedSavingsBatchV1 {
            schema_version: 1,
            kind: "lean-ctx.savings-batch".into(),
            created_at: created_at.into(),
            lean_ctx_version: "test".into(),
            agent_id: format!("agent-{signer}"),
            period: "all".into(),
            first_entry_hash: "genesis".into(),
            last_entry_hash: "head".into(),
            chain_valid: true,
            totals: BatchTotals {
                total_events: 1,
                saved_tokens: net,
                net_saved_tokens: net,
                saved_usd: usd,
                bounce_tokens: 0,
                bounce_events: 0,
                tokenizers: vec!["o200k_base".into()],
                by_model: vec![("claude-opus".into(), net, usd)],
                by_tool: vec![("ctx_read".into(), net)],
            },
            signer_public_key: Some(signer.into()),
            signature: Some("sig".into()),
        }
    }

    fn write_lines(dir: &Path, file: &str, batches: &[SignedSavingsBatchV1]) {
        let body = batches
            .iter()
            .map(|b| serde_json::to_string(b).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(dir.join(file), body + "\n").unwrap();
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "leanctx_savings_summary_{tag}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn latest_batch_per_signer_is_not_double_counted() {
        let dir = temp_dir("nodouble");
        // Signer A re-snapshots twice (1000 → 3000); only the latest must count.
        write_lines(
            &dir,
            "savings_aaaaaaaaaaaaaaaa.jsonl",
            &[
                batch("aaaaaaaaaaaaaaaa", 1000, 0.01, "2026-06-01T00:00:00Z"),
                batch("aaaaaaaaaaaaaaaa", 3000, 0.03, "2026-06-08T00:00:00Z"),
            ],
        );
        // Signer B has a single snapshot.
        write_lines(
            &dir,
            "savings_bbbbbbbbbbbbbbbb.jsonl",
            &[batch(
                "bbbbbbbbbbbbbbbb",
                2000,
                0.02,
                "2026-06-07T00:00:00Z",
            )],
        );

        let s = aggregate(&dir);
        assert_eq!(s.member_count, 2);
        // 3000 (A latest) + 2000 (B) = 5000 — NOT 1000+3000+2000.
        assert_eq!(s.totals.net_saved_tokens, 5000);
        // by_member sorted descending by net tokens.
        assert_eq!(s.by_member[0].net_saved_tokens, 3000);
        assert_eq!(s.by_member[1].net_saved_tokens, 2000);
        // model breakdown is summed over members' latest batches.
        assert_eq!(s.by_model[0].model, "claude-opus");
        assert_eq!(s.by_model[0].saved_tokens, 5000);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_or_missing_store_is_zeroed() {
        let missing = std::env::temp_dir().join("leanctx_savings_summary_does_not_exist_xyz");
        let _ = std::fs::remove_dir_all(&missing);
        let s = aggregate(&missing);
        assert_eq!(s.member_count, 0);
        assert_eq!(s.totals.net_saved_tokens, 0);
        assert!(s.by_member.is_empty());
    }

    #[test]
    fn non_savings_files_are_ignored() {
        let dir = temp_dir("ignore");
        std::fs::write(dir.join("audit.jsonl"), "{\"not\":\"a batch\"}\n").unwrap();
        std::fs::write(dir.join("README.md"), "hello\n").unwrap();
        write_lines(
            &dir,
            "savings_cccccccccccccccc.jsonl",
            &[batch(
                "cccccccccccccccc",
                700,
                0.007,
                "2026-06-08T00:00:00Z",
            )],
        );
        let s = aggregate(&dir);
        assert_eq!(s.member_count, 1);
        assert_eq!(s.totals.net_saved_tokens, 700);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
