use crate::core::sidecar_transport::types::{
    Acknowledgement, SignedPolicy, TransportError, TransportHealth,
};
use lean_ctx_protocol::{
    EvidenceGapClosedV1, EvidenceGapOpenedV1, ExperimentAssignmentV1, SavingsObservationV1,
};

/// The communication contract between OSS Runtime and Enterprise Sidecar.
/// OSS ships with NoopTransport. Enterprise Sidecar provides real implementation.
#[async_trait::async_trait]
pub trait RuntimeSidecarTransport: Send + Sync {
    /// Send a savings observation to the sidecar for forwarding to the platform.
    async fn emit_observation(
        &self,
        observation: SavingsObservationV1,
    ) -> Result<Acknowledgement, TransportError>;

    /// Send an evidence gap marker to the sidecar.
    async fn emit_gap_opened(
        &self,
        gap: EvidenceGapOpenedV1,
    ) -> Result<Acknowledgement, TransportError>;

    /// Send a gap closure to the sidecar.
    async fn emit_gap_closed(
        &self,
        gap: EvidenceGapClosedV1,
    ) -> Result<Acknowledgement, TransportError>;

    /// Receive the latest signed policy bundle from the sidecar.
    async fn receive_policy(&self) -> Result<Option<SignedPolicy>, TransportError>;

    /// Receive experiment assignments from the sidecar.
    async fn receive_assignments(&self) -> Result<Vec<ExperimentAssignmentV1>, TransportError>;

    /// Check transport health and connectivity.
    async fn health(&self) -> TransportHealth;
}
