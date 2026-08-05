use crate::core::sidecar_transport::transport_trait::RuntimeSidecarTransport;
use crate::core::sidecar_transport::types::{
    Acknowledgement, SignedPolicy, TransportError, TransportHealth,
};
use lean_ctx_protocol::{
    EvidenceGapClosedV1, EvidenceGapOpenedV1, ExperimentAssignmentV1, SavingsObservationV1,
};

/// No-op transport for standalone OSS mode (no sidecar connected).
/// All emit calls succeed silently. All receive calls return empty.
pub struct NoopTransport;

impl NoopTransport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RuntimeSidecarTransport for NoopTransport {
    async fn emit_observation(
        &self,
        _: SavingsObservationV1,
    ) -> Result<Acknowledgement, TransportError> {
        Ok(Acknowledgement {
            sequence: 0,
            accepted: true,
        })
    }

    async fn emit_gap_opened(
        &self,
        _: EvidenceGapOpenedV1,
    ) -> Result<Acknowledgement, TransportError> {
        Ok(Acknowledgement {
            sequence: 0,
            accepted: true,
        })
    }

    async fn emit_gap_closed(
        &self,
        _: EvidenceGapClosedV1,
    ) -> Result<Acknowledgement, TransportError> {
        Ok(Acknowledgement {
            sequence: 0,
            accepted: true,
        })
    }

    async fn receive_policy(&self) -> Result<Option<SignedPolicy>, TransportError> {
        Ok(None)
    }

    async fn receive_assignments(&self) -> Result<Vec<ExperimentAssignmentV1>, TransportError> {
        Ok(Vec::new())
    }

    async fn health(&self) -> TransportHealth {
        TransportHealth::Disconnected
    }
}

#[cfg(test)]
mod tests {
    use super::{NoopTransport, RuntimeSidecarTransport, TransportHealth};
    use lean_ctx_protocol::{MoneyV1, SavingsObservationV1, UsageBreakdownV1};

    #[tokio::test]
    async fn reports_disconnected_health() {
        assert_eq!(
            NoopTransport::new().health().await,
            TransportHealth::Disconnected
        );
    }

    #[tokio::test]
    async fn accepts_observations() {
        let acknowledgement = NoopTransport::new()
            .emit_observation(observation())
            .await
            .expect("no-op transport should accept observations");

        assert!(acknowledgement.accepted);
        assert_eq!(acknowledgement.sequence, 0);
    }

    #[tokio::test]
    async fn returns_no_policy() {
        let policy = NoopTransport::new()
            .receive_policy()
            .await
            .expect("no-op transport should return successfully");

        assert!(policy.is_none());
    }

    #[tokio::test]
    async fn returns_no_assignments() {
        let assignments = NoopTransport::new()
            .receive_assignments()
            .await
            .expect("no-op transport should return successfully");

        assert!(assignments.is_empty());
    }

    fn observation() -> SavingsObservationV1 {
        SavingsObservationV1 {
            observation_id: "observation-1".to_owned(),
            provider_id: "provider".to_owned(),
            model_id: "model".to_owned(),
            original_usage: UsageBreakdownV1::default(),
            actual_usage: UsageBreakdownV1::default(),
            local_cost_estimate: MoneyV1 {
                currency: "USD".to_owned(),
                coefficient: 0,
                scale: 0,
            },
            evidence_refs: Vec::new(),
            observed_at: "2026-01-01T00:00:00Z".to_owned(),
            runtime_version: "1.0.0".to_owned(),
            sequence_number: 1,
        }
    }
}
