use super::helpers::GWT_IGNITION_Z;
use super::types::{ContextLedger, LedgerEntry};

#[derive(Debug, Clone)]
pub struct ReinjectionAction {
    pub path: String,
    pub current_mode: String,
    pub new_mode: String,
    pub tokens_freed: usize,
}

#[derive(Debug, Clone)]
pub struct ReinjectionPlan {
    pub actions: Vec<ReinjectionAction>,
    pub total_tokens_freed: usize,
    pub new_utilization: f64,
}

impl ContextLedger {
    pub fn reinjection_plan(
        &self,
        intent: &super::super::intent_engine::StructuredIntent,
        target_utilization: f64,
    ) -> ReinjectionPlan {
        let current_util = self.total_tokens_sent as f64 / self.window_size as f64;
        if current_util <= target_utilization {
            return ReinjectionPlan {
                actions: Vec::new(),
                total_tokens_freed: 0,
                new_utilization: current_util,
            };
        }

        let tokens_to_free =
            self.total_tokens_sent - (self.window_size as f64 * target_utilization) as usize;

        let target_set: std::collections::HashSet<&str> = intent
            .targets
            .iter()
            .map(std::string::String::as_str)
            .collect();

        let mut candidates: Vec<(usize, &LedgerEntry)> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !target_set.iter().any(|t| e.path.contains(t)))
            .collect();

        candidates.sort_by(|a, b| {
            let a_phi = a.1.phi.unwrap_or(0.0);
            let b_phi = b.1.phi.unwrap_or(0.0);
            a_phi
                .partial_cmp(&b_phi)
                .unwrap_or_else(|| a.1.timestamp.cmp(&b.1.timestamp))
        });

        let mut actions = Vec::new();
        let mut freed = 0usize;

        for (_, entry) in &candidates {
            if freed >= tokens_to_free {
                break;
            }
            if let Some((new_mode, new_tokens)) = downgrade_mode(&entry.mode, entry.sent_tokens) {
                let saving = entry.sent_tokens.saturating_sub(new_tokens);
                if saving > 0 {
                    actions.push(ReinjectionAction {
                        path: entry.path.clone(),
                        current_mode: entry.mode.clone(),
                        new_mode,
                        tokens_freed: saving,
                    });
                    freed += saving;
                }
            }
        }

        let new_sent = self.total_tokens_sent.saturating_sub(freed);
        let new_utilization = new_sent as f64 / self.window_size as f64;

        ReinjectionPlan {
            actions,
            total_tokens_freed: freed,
            new_utilization,
        }
    }
}

pub(super) fn downgrade_mode(current_mode: &str, current_tokens: usize) -> Option<(String, usize)> {
    match current_mode {
        "full" => Some(("signatures".to_string(), current_tokens / 5)),
        "aggressive" => Some(("signatures".to_string(), current_tokens / 3)),
        "signatures" => Some(("map".to_string(), current_tokens / 2)),
        "map" => Some(("reference".to_string(), current_tokens / 4)),
        _ => None,
    }
}

/// Resolve the Global-Workspace ignition z-score threshold (#6): the
/// `LEAN_CTX_GWT_IGNITION_Z` env override (must be > 0) wins, else the default
/// [`GWT_IGNITION_Z`]. Deterministic for a given environment.
pub(super) fn ignition_z_threshold() -> f64 {
    std::env::var("LEAN_CTX_GWT_IGNITION_Z")
        .ok()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| *v > 0.0)
        .unwrap_or(GWT_IGNITION_Z)
}

impl Default for ContextLedger {
    fn default() -> Self {
        Self::new()
    }
}
