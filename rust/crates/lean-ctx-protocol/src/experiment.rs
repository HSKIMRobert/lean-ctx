use crate::MoneyV1;
use serde::{Deserialize, Serialize};

/// Experiment arm assignment — signed by proprietary platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperimentArm {
    Control,
    Optimized,
    Shadow,
}

/// Signed assignment from sidecar. Runtime executes, never decides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperimentAssignmentV1 {
    pub experiment_id: String,
    pub subject_id: String,
    pub arm: ExperimentArm,
    pub configuration_ref: String,
    pub expires_at: String,
    pub max_incremental_cost: MoneyV1,
    pub allowed_providers: Vec<String>,
    pub allowed_models: Vec<String>,
    pub data_classification: DataClassification,
    pub side_effect_policy: SideEffectPolicy,
    pub kill_switch_ref: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideEffectPolicy {
    NoSideEffects,
    ReadOnly,
    AllowWrites,
}

#[cfg(test)]
mod tests {
    use crate::{
        DataClassification, ExperimentArm, ExperimentAssignmentV1, MoneyV1, SideEffectPolicy,
    };

    #[test]
    fn serialization_round_trip() {
        let assignment = ExperimentAssignmentV1 {
            experiment_id: "exp-1".to_owned(),
            subject_id: "subject-1".to_owned(),
            arm: ExperimentArm::Optimized,
            configuration_ref: "config:1".to_owned(),
            expires_at: "2026-01-01T00:00:00Z".to_owned(),
            max_incremental_cost: MoneyV1 {
                currency: "USD".to_owned(),
                coefficient: 25,
                scale: 4,
            },
            allowed_providers: vec!["provider".to_owned()],
            allowed_models: vec!["model".to_owned()],
            data_classification: DataClassification::Internal,
            side_effect_policy: SideEffectPolicy::ReadOnly,
            kill_switch_ref: "kill:1".to_owned(),
            signature: "signature".to_owned(),
        };
        let json = serde_json::to_string(&assignment).expect("assignment should serialize");
        let decoded = serde_json::from_str(&json).expect("assignment should deserialize");
        assert_eq!(assignment, decoded);
    }
}
