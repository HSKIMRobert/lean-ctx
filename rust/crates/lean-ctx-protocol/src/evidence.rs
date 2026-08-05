use serde::{Deserialize, Serialize};

/// Typed evidence reference (not just a string).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRefV1 {
    pub kind: EvidenceKind,
    pub uri: String,
    pub digest: String,
    pub signature_status: SignatureStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceKind {
    ProviderReceipt,
    RuntimeLog,
    SignedBatch,
    QualityMeasurement,
    ExperimentOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureStatus {
    Verified,
    Unverified,
    NotSigned,
}

#[cfg(test)]
mod tests {
    use crate::{EvidenceKind, EvidenceRefV1, SignatureStatus};

    #[test]
    fn serialization_round_trip() {
        let evidence = EvidenceRefV1 {
            kind: EvidenceKind::ProviderReceipt,
            uri: "urn:receipt:1".to_owned(),
            digest: "sha256:abc".to_owned(),
            signature_status: SignatureStatus::Verified,
        };
        let json = serde_json::to_string(&evidence).expect("evidence should serialize");
        let decoded = serde_json::from_str(&json).expect("evidence should deserialize");
        assert_eq!(evidence, decoded);
    }
}
