//! Usage-based metering — OSS stub (ADR-023).
//!
//! The full metering implementation (commercial enforcement, settlement
//! authority checks) lives in `lean-ctx-enterprise/commercial-core`.
//! This stub preserves the public API surface so OSS code compiles.

use serde::{Deserialize, Serialize};

use crate::core::savings_ledger::{RoiReport, roi_report};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub schema_version: u32,
    pub period: String,
    pub created_at: String,
    pub agent_id: String,
    pub metered_events: usize,
    pub net_saved_tokens: u64,
    pub saved_usd: f64,
    pub last_entry_hash: String,
    pub chain_valid: bool,
    pub signed: bool,
}

impl Usage {
    pub const SCHEMA_VERSION: u32 = 1;

    #[must_use]
    pub fn from_roi(roi: &RoiReport) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            period: roi.period.clone(),
            created_at: roi.created_at.clone(),
            agent_id: roi.agent_id.clone(),
            metered_events: roi.total_events,
            net_saved_tokens: roi.net_saved_tokens,
            saved_usd: roi.saved_usd,
            last_entry_hash: roi.last_entry_hash.clone(),
            chain_valid: roi.chain_valid,
            signed: roi.signed,
        }
    }

    #[must_use]
    pub fn is_billable(&self) -> bool {
        false
    }

    #[must_use]
    pub fn source_integrity_verified(&self) -> bool {
        false
    }

    #[must_use]
    pub fn headline(&self) -> String {
        format!(
            "Usage[{}]: {} events, {} net tokens, ${:.4} (OSS — metering stub)",
            self.period, self.metered_events, self.net_saved_tokens, self.saved_usd,
        )
    }
}

#[must_use]
pub fn metered_usage(agent_id: &str) -> Usage {
    Usage::from_roi(&roi_report(agent_id))
}
