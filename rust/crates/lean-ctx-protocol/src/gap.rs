use serde::{Deserialize, Serialize};

/// Gap lifecycle — append-only audit events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapReason {
    BufferFull,
    NetworkTimeout,
    ProcessCrash,
    SidecarUnreachable,
    DiskFull,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceGapOpenedV1 {
    pub gap_id: String,
    pub tenant_id: String,
    pub source_instance_id: String,
    pub first_missing_sequence: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_missing_sequence: Option<u64>,
    pub started_at: String,
    pub detected_at: String,
    pub reason: GapReason,
    pub affected_event_types: Vec<String>,
    pub runtime_version: String,
    pub previous_evidence_hash: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceGapClosedV1 {
    pub gap_id: String,
    pub last_missing_sequence: u64,
    pub ended_at: String,
    pub resolved_at: String,
    pub total_missing_events: u64,
    pub billing_period_status: BillingPeriodStatus,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingPeriodStatus {
    Complete,
    Incomplete,
    NonBillable,
}

#[cfg(test)]
mod tests {
    use crate::{BillingPeriodStatus, EvidenceGapClosedV1, EvidenceGapOpenedV1, GapReason};

    #[test]
    fn opened_serialization_round_trip() {
        let opened = EvidenceGapOpenedV1 {
            gap_id: "gap-1".to_owned(),
            tenant_id: "tenant-1".to_owned(),
            source_instance_id: "runtime-1".to_owned(),
            first_missing_sequence: 7,
            last_missing_sequence: None,
            started_at: "2026-01-01T00:00:00Z".to_owned(),
            detected_at: "2026-01-01T00:01:00Z".to_owned(),
            reason: GapReason::NetworkTimeout,
            affected_event_types: vec!["observation".to_owned()],
            runtime_version: "1.0.0".to_owned(),
            previous_evidence_hash: "sha256:abc".to_owned(),
            signature: "signature".to_owned(),
        };
        let json = serde_json::to_string(&opened).expect("opened gap should serialize");
        let decoded = serde_json::from_str(&json).expect("opened gap should deserialize");
        assert_eq!(opened, decoded);
    }

    #[test]
    fn closed_serialization_round_trip() {
        let closed = EvidenceGapClosedV1 {
            gap_id: "gap-1".to_owned(),
            last_missing_sequence: 9,
            ended_at: "2026-01-01T00:02:00Z".to_owned(),
            resolved_at: "2026-01-01T00:03:00Z".to_owned(),
            total_missing_events: 3,
            billing_period_status: BillingPeriodStatus::Incomplete,
            signature: "signature".to_owned(),
        };
        let json = serde_json::to_string(&closed).expect("closed gap should serialize");
        let decoded = serde_json::from_str(&json).expect("closed gap should deserialize");
        assert_eq!(closed, decoded);
    }
}
