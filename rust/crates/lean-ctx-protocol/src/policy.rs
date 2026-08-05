use serde::{Deserialize, Serialize};

/// Policy class — determines fail behavior on expiry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyCriticality {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpiryBehavior {
    FailClosed,
    FailOpen,
    GracePeriod,
}

/// Extended policy rule with classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyClassification {
    pub criticality: PolicyCriticality,
    pub expiry_behavior: ExpiryBehavior,
    pub grace_period_seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_policy_ref: Option<String>,
    pub last_known_good_allowed: bool,
}

#[cfg(test)]
mod tests {
    use crate::{ExpiryBehavior, PolicyClassification, PolicyCriticality};

    #[test]
    fn serialization_round_trip() {
        let classification = PolicyClassification {
            criticality: PolicyCriticality::Critical,
            expiry_behavior: ExpiryBehavior::FailClosed,
            grace_period_seconds: 0,
            fallback_policy_ref: None,
            last_known_good_allowed: false,
        };
        let json = serde_json::to_string(&classification).expect("policy should serialize");
        let decoded = serde_json::from_str(&json).expect("policy should deserialize");
        assert_eq!(classification, decoded);
    }
}
