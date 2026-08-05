use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Acknowledgement {
    pub sequence: u64,
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPolicy {
    pub policy_version: String,
    pub payload: Vec<u8>,
    pub signature: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportHealth {
    Connected,
    Disconnected,
    Degraded { reason: String },
}

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("transport disconnected")]
    Disconnected,
    #[error("timeout after {0}ms")]
    Timeout(u64),
    #[error("sidecar rejected: {0}")]
    Rejected(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}
