use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::core::a2a::dlq::DeadLetter;
use crate::core::a2a_transport::TransportEnvelopeV1;

const DEFAULT_MAX_PAYLOAD_BYTES: usize = 2_000_000;
const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteTransportConfig {
    pub endpoint_url: String,
    pub timeout: Duration,
    pub max_payload_bytes: usize,
    pub auth_token: Option<String>,
    pub retry_count: u8,
    pub retry_delay: Duration,
}

impl Default for RemoteTransportConfig {
    fn default() -> Self {
        Self {
            endpoint_url: String::new(),
            timeout: Duration::from_secs(30),
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
            auth_token: None,
            retry_count: 2,
            retry_delay: Duration::from_secs(1),
        }
    }
}

impl RemoteTransportConfig {
    pub fn validate(&self) -> Result<(), String> {
        let url = reqwest::Url::parse(&self.endpoint_url)
            .map_err(|error| format!("invalid endpoint_url: {error}"))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err("endpoint_url must be an absolute HTTP(S) URL".to_string());
        }
        if self.timeout.is_zero() {
            return Err("timeout must be greater than zero".to_string());
        }
        if self.max_payload_bytes == 0 {
            return Err("max_payload_bytes must be greater than zero".to_string());
        }
        if self.auth_token.as_ref().is_some_and(String::is_empty) {
            return Err("auth_token must not be empty".to_string());
        }
        Ok(())
    }

    fn delivery_url(&self) -> Result<reqwest::Url, String> {
        self.validate()?;
        let mut url = reqwest::Url::parse(&self.endpoint_url)
            .map_err(|error| format!("invalid endpoint_url: {error}"))?;
        let path = format!("{}/a2a/deliver", url.path().trim_end_matches('/'));
        url.set_path(&path);
        url.set_query(None);
        url.set_fragment(None);
        Ok(url)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTransport {
    config: RemoteTransportConfig,
    #[serde(skip, default = "reqwest::Client::new")]
    client: reqwest::Client,
}

impl RemoteTransport {
    pub fn new(config: RemoteTransportConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            client: reqwest::Client::new(),
        })
    }

    pub async fn deliver(
        &self,
        envelope: &TransportEnvelopeV1,
    ) -> Result<DeliveryReceipt, TransportError> {
        let body = serialize_and_validate(envelope, self.config.max_payload_bytes)?;
        let envelope_id = envelope_id(&body);
        let delivery_url = self
            .config
            .delivery_url()
            .map_err(TransportError::SerializationError)?;
        let started_at = Instant::now();

        for attempt in 0..=self.config.retry_count {
            let mut request = self
                .client
                .post(delivery_url.clone())
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .timeout(self.config.timeout)
                .body(body.clone());
            if let Some(token) = self.config.auth_token.as_deref() {
                request = request.bearer_auth(token);
            }

            match request.send().await {
                Ok(response) if response.status().is_success() => {
                    return Ok(DeliveryReceipt {
                        envelope_id,
                        delivered_at: Utc::now(),
                        remote_status: response.status().as_u16(),
                        round_trip_ms: elapsed_millis(started_at),
                    });
                }
                Ok(response) if response.status().is_server_error() => {
                    if attempt == self.config.retry_count {
                        return Err(TransportError::Exhausted(self.config.retry_count));
                    }
                }
                Ok(response) => {
                    let status = response.status();
                    let error_body = read_error_body(response).await;
                    if status.is_client_error() {
                        enqueue_permanent_failure(
                            &envelope_id,
                            envelope,
                            &body,
                            status.as_u16(),
                            &error_body,
                            attempt.saturating_add(1),
                            &self.config.endpoint_url,
                        );
                    }
                    return Err(TransportError::RemoteError(status.as_u16(), error_body));
                }
                Err(error) if error.is_timeout() && self.config.retry_count == 0 => {
                    return Err(TransportError::Timeout);
                }
                Err(_) if attempt == self.config.retry_count => {
                    return Err(TransportError::Exhausted(self.config.retry_count));
                }
                Err(_) => {}
            }

            tokio::time::sleep(self.config.retry_delay).await;
        }

        Err(TransportError::Exhausted(self.config.retry_count))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryReceipt {
    pub envelope_id: String,
    pub delivered_at: DateTime<Utc>,
    pub remote_status: u16,
    pub round_trip_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    #[error("payload too large: {0} bytes")]
    PayloadTooLarge(usize),
    #[error("transport timed out")]
    Timeout,
    #[error("remote returned HTTP {0}: {1}")]
    RemoteError(u16, String),
    #[error("serialization failed: {0}")]
    SerializationError(String),
    #[error("delivery exhausted after {0} retries")]
    Exhausted(u8),
}

fn serialize_and_validate(
    envelope: &TransportEnvelopeV1,
    max_payload_bytes: usize,
) -> Result<Vec<u8>, TransportError> {
    let body = serde_json::to_vec(envelope)
        .map_err(|error| TransportError::SerializationError(error.to_string()))?;
    if body.len() > max_payload_bytes {
        return Err(TransportError::PayloadTooLarge(body.len()));
    }
    Ok(body)
}

fn envelope_id(body: &[u8]) -> String {
    format!("envelope:{}", blake3::hash(body).to_hex())
}

fn elapsed_millis(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

async fn read_error_body(response: reqwest::Response) -> String {
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk else {
            break;
        };
        let remaining = MAX_ERROR_BODY_BYTES.saturating_sub(body.len());
        body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if body.len() == MAX_ERROR_BODY_BYTES {
            break;
        }
    }
    String::from_utf8_lossy(&body).into_owned()
}

fn enqueue_permanent_failure(
    envelope_id: &str,
    envelope: &TransportEnvelopeV1,
    body: &[u8],
    status: u16,
    error_body: &str,
    attempts: u8,
    endpoint_url: &str,
) {
    let failed_at = Utc::now().to_rfc3339();
    let target_agent = envelope.recipient.as_deref().unwrap_or(endpoint_url);
    crate::core::ocla::health::dead_letter_queue().enqueue(DeadLetter {
        id: envelope_id.to_string(),
        original_message: String::from_utf8_lossy(body).into_owned(),
        target_agent: target_agent.to_string(),
        error: format!("HTTP {status}: {error_body}"),
        attempts,
        first_failed_at: failed_at.clone(),
        last_failed_at: failed_at,
    });
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::core::a2a_transport::{AgentIdentityV1, TransportContentType};

    fn envelope(payload: &str) -> TransportEnvelopeV1 {
        TransportEnvelopeV1 {
            format_version: 1,
            sent_at: Utc::now(),
            sender: AgentIdentityV1 {
                agent_id: "sender".to_string(),
                agent_type: "test".to_string(),
                daemon_fingerprint: "fingerprint".to_string(),
                capabilities: Vec::new(),
            },
            recipient: Some("recipient".to_string()),
            content_type: TransportContentType::A2AMessage,
            payload_json: payload.to_string(),
            signature: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn default_config_has_bounded_transport_values() {
        let config = RemoteTransportConfig {
            endpoint_url: "https://agent.example/api/".to_string(),
            ..RemoteTransportConfig::default()
        };

        assert!(config.validate().is_ok());
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_payload_bytes, 2_000_000);
        assert_eq!(config.retry_count, 2);
        assert_eq!(config.retry_delay, Duration::from_secs(1));
        assert_eq!(
            config.delivery_url().expect("valid URL").as_str(),
            "https://agent.example/api/a2a/deliver"
        );
    }

    #[test]
    fn config_validation_rejects_unbounded_or_unsupported_values() {
        let zero_timeout = RemoteTransportConfig {
            endpoint_url: "https://agent.example".to_string(),
            timeout: Duration::ZERO,
            ..RemoteTransportConfig::default()
        };
        let unsupported_scheme = RemoteTransportConfig {
            endpoint_url: "file:///tmp/agent".to_string(),
            ..RemoteTransportConfig::default()
        };

        assert!(zero_timeout.validate().is_err());
        assert!(unsupported_scheme.validate().is_err());
    }

    #[test]
    fn payload_size_limit_reports_serialized_size() {
        let envelope = envelope("payload");
        let serialized = serde_json::to_vec(&envelope).expect("serializable envelope");
        let error = serialize_and_validate(&envelope, serialized.len() - 1)
            .expect_err("payload must exceed configured limit");

        assert_eq!(error, TransportError::PayloadTooLarge(serialized.len()));
    }

    #[test]
    fn transport_error_variants_are_serializable_and_distinct() {
        let variants = [
            TransportError::PayloadTooLarge(10),
            TransportError::Timeout,
            TransportError::RemoteError(400, "bad request".to_string()),
            TransportError::SerializationError("invalid JSON".to_string()),
            TransportError::Exhausted(2),
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).expect("serialize error");
            let decoded = serde_json::from_str(&json).expect("deserialize error");
            assert_eq!(variant, decoded);
        }
    }

    #[test]
    fn receipt_round_trips_without_losing_delivery_fields() {
        let receipt = DeliveryReceipt {
            envelope_id: "envelope:abc".to_string(),
            delivered_at: Utc::now(),
            remote_status: 202,
            round_trip_ms: 17,
        };

        let json = serde_json::to_string(&receipt).expect("serialize receipt");
        let decoded: DeliveryReceipt = serde_json::from_str(&json).expect("deserialize receipt");
        assert_eq!(decoded, receipt);
    }

    #[test]
    fn envelope_ids_are_deterministic_and_content_addressed() {
        let first = serialize_and_validate(&envelope("one"), usize::MAX).expect("serialize");
        let first_again = first.clone();
        let second = serialize_and_validate(&envelope("two"), usize::MAX).expect("serialize");

        assert_eq!(envelope_id(&first), envelope_id(&first_again));
        assert_ne!(envelope_id(&first), envelope_id(&second));
    }
}
