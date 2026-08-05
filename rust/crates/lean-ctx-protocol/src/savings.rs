use crate::{EvidenceRefV1, MoneyV1, UsageBreakdownV1};
use serde::{Deserialize, Serialize};

/// What OSS emits. NOT VerifiedSavings (that's proprietary).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavingsObservationV1 {
    pub observation_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub original_usage: UsageBreakdownV1,
    pub actual_usage: UsageBreakdownV1,
    pub local_cost_estimate: MoneyV1,
    pub evidence_refs: Vec<EvidenceRefV1>,
    pub observed_at: String,
    pub runtime_version: String,
    pub sequence_number: u64,
}

#[cfg(test)]
mod tests {
    use crate::{MoneyV1, SavingsObservationV1, UsageBreakdownV1};

    #[test]
    fn serialization_round_trip() {
        let observation = SavingsObservationV1 {
            observation_id: "obs-1".to_owned(),
            provider_id: "provider".to_owned(),
            model_id: "model".to_owned(),
            original_usage: UsageBreakdownV1::default(),
            actual_usage: UsageBreakdownV1::default(),
            local_cost_estimate: MoneyV1 {
                currency: "USD".to_owned(),
                coefficient: 1,
                scale: 4,
            },
            evidence_refs: Vec::new(),
            observed_at: "2026-01-01T00:00:00Z".to_owned(),
            runtime_version: "1.0.0".to_owned(),
            sequence_number: 1,
        };
        let json = serde_json::to_string(&observation).expect("observation should serialize");
        let decoded = serde_json::from_str(&json).expect("observation should deserialize");
        assert_eq!(observation, decoded);
    }
}
