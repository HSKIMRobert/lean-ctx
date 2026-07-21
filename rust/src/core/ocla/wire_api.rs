//! Axum REST projection for the public OCLA wire contract.

use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};
use serde::Serialize;
use serde_json::{Value, json};

use super::health::{SystemHealth, check_system_health};
use super::{
    CanonicalTokenEnvelopeV1, OCLA_API_VERSION, OclaCapability, OclaCapabilityKind, OclaRegistry,
};
use crate::core::ocla::wire::decode_envelope;

/// Builds the stateless OCLA REST router for merging into an Axum application.
pub fn ocla_router() -> Router {
    Router::new()
        .route("/ocla/v1/health", get(health))
        .route("/ocla/v1/capabilities", get(capabilities))
        .route("/ocla/v1/envelope", post(envelope))
        .route("/ocla/v1/envelope/batch", post(envelope_batch))
        .route("/ocla/v1/agents", get(agents))
        .route("/ocla/v1/metrics", get(metrics))
        .route("/ocla/v1/ledger/summary", get(ledger_summary))
}

async fn health() -> Json<SystemHealth> {
    Json(check_system_health())
}

#[derive(Serialize)]
struct CapabilitiesResponse {
    version: &'static str,
    capabilities: Vec<OclaCapability>,
}

async fn capabilities() -> Json<CapabilitiesResponse> {
    let registry = OclaRegistry::global();
    let capabilities = vec![
        registry.observation_hook.capability(),
        registry.usage_sink.capability(),
        registry.metrics_exporter.capability(),
        registry.savings_ledger.capability(),
        registry.intent_classifier.capability(),
        registry.outcome_tracker.capability(),
        registry.compression_provider.capability(),
        registry.response_optimizer.capability(),
        registry.model_router.capability(),
        registry.efficiency_analyzer.capability(),
        registry.config_tuner.capability(),
        registry.experiment_runner.capability(),
        registry.connector_scheduler.capability(),
        registry.agent_gateway.capability(),
    ];
    debug_assert_eq!(capabilities.len(), OclaCapabilityKind::ALL.len());

    Json(CapabilitiesResponse {
        version: OCLA_API_VERSION,
        capabilities,
    })
}

async fn envelope(
    body: String,
) -> Result<Json<CanonicalTokenEnvelopeV1>, (StatusCode, Json<Value>)> {
    decode_envelope(&body).map(Json).map_err(invalid_request)
}

#[derive(Serialize)]
struct BatchEnvelopeResult {
    valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    envelope: Option<CanonicalTokenEnvelopeV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn envelope_batch(Json(envelopes): Json<Vec<Value>>) -> Json<Vec<BatchEnvelopeResult>> {
    let results = envelopes
        .into_iter()
        .map(|envelope| match serde_json::to_string(&envelope) {
            Ok(json) => match decode_envelope(&json) {
                Ok(envelope) => BatchEnvelopeResult {
                    valid: true,
                    envelope: Some(envelope),
                    error: None,
                },
                Err(error) => BatchEnvelopeResult {
                    valid: false,
                    envelope: None,
                    error: Some(error.to_string()),
                },
            },
            Err(error) => BatchEnvelopeResult {
                valid: false,
                envelope: None,
                error: Some(error.to_string()),
            },
        })
        .collect();
    Json(results)
}

#[derive(Serialize)]
struct AgentsResponse {
    agents: Vec<Value>,
}

async fn agents() -> Json<AgentsResponse> {
    Json(AgentsResponse { agents: Vec::new() })
}

#[derive(Serialize)]
struct MetricsResponse {
    total_events: usize,
    saved_tokens: u64,
    saved_usd: f64,
    trait_adoption_count: usize,
}

async fn metrics() -> Json<MetricsResponse> {
    let summary = crate::core::savings_ledger::summary();
    Json(MetricsResponse {
        total_events: summary.total_events,
        saved_tokens: summary.saved_tokens,
        saved_usd: summary.saved_usd,
        trait_adoption_count: OclaCapabilityKind::ALL.len(),
    })
}

#[derive(Serialize)]
struct LedgerSummaryResponse {
    events: usize,
    tokens: u64,
    usd: f64,
}

async fn ledger_summary() -> Json<LedgerSummaryResponse> {
    let summary = crate::core::savings_ledger::summary();
    Json(LedgerSummaryResponse {
        events: summary.total_events,
        tokens: summary.saved_tokens,
        usd: summary.saved_usd,
    })
}

fn invalid_request(error: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": error.to_string()})),
    )
}

#[cfg(test)]
mod tests {
    use super::{CanonicalTokenEnvelopeV1, OCLA_API_VERSION, ocla_router};
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::{Request, StatusCode, header};
    use serde_json::{Value, json};
    use tower::ServiceExt;

    fn request_context() -> super::super::OclaRequestContext {
        super::super::OclaRequestContext {
            request_id: "request-1".into(),
            session_id: "session-1".into(),
            agent_id: "agent-1".into(),
            content_ref: "blake3:content".into(),
            tenant_id: None,
        }
    }

    fn valid_envelope() -> CanonicalTokenEnvelopeV1 {
        CanonicalTokenEnvelopeV1 {
            schema_version: super::super::CANONICAL_TOKEN_ENVELOPE_SCHEMA_VERSION,
            context: request_context(),
            surface: super::super::TokenEnvelopeSurface::Proxy,
            direction: super::super::TokenFlowDirection::Input,
            provider: "openai".into(),
            model: "gpt-5".into(),
            token_balance: super::super::TokenBalanceV1 {
                original_tokens: 100,
                materialized_tokens: 80,
                delivered_tokens: 60,
                provider_billed_tokens: 60,
            },
            route_ref: Some("route-1".into()),
            policy_ref: None,
            idempotency_key: "request-1:input".into(),
        }
    }

    async fn json_response(response: axum::response::Response) -> Value {
        let body = to_bytes(response.into_body(), 1_000_000)
            .await
            .expect("response body");
        serde_json::from_slice(&body).expect("JSON response")
    }

    #[tokio::test]
    async fn health_endpoint_returns_full_report() {
        let response = ocla_router()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/ocla/v1/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = json_response(response).await;
        assert_eq!(body["version"], OCLA_API_VERSION);
        assert_eq!(body["components"].as_array().expect("components").len(), 17);
        assert!(body.get("overall").is_some());
        assert!(body.get("uptime_seconds").is_some());
    }

    #[tokio::test]
    async fn capabilities_endpoint_lists_all_fourteen_statuses() {
        let response = ocla_router()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/ocla/v1/capabilities")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = json_response(response).await;
        assert_eq!(body["version"], OCLA_API_VERSION);
        assert_eq!(body["capabilities"].as_array().expect("list").len(), 14);
        assert!(
            body["capabilities"]
                .as_array()
                .expect("list")
                .iter()
                .all(|capability| capability["status"] == "available")
        );
    }

    #[tokio::test]
    async fn envelope_endpoint_decodes_valid_json_and_rejects_invalid_json() {
        let wire = serde_json::to_string(&valid_envelope()).expect("envelope JSON");
        let response = ocla_router()
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocla/v1/envelope")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(wire))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_response(response).await, json!(valid_envelope()));

        let response = ocla_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocla/v1/envelope")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"schema_version":99}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn ledger_summary_endpoint_returns_events_tokens_and_usd() {
        let response = ocla_router()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/ocla/v1/ledger/summary")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = json_response(response).await;
        assert!(body.get("events").is_some());
        assert!(body.get("tokens").is_some());
        assert!(body.get("usd").is_some());
    }

    #[tokio::test]
    async fn agents_endpoint_returns_registered_agents_schema() {
        let response = ocla_router()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/ocla/v1/agents")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_response(response).await, json!({"agents": []}));
    }

    #[tokio::test]
    async fn metrics_endpoint_returns_key_ocla_metrics() {
        let response = ocla_router()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/ocla/v1/metrics")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = json_response(response).await;
        assert!(body.get("total_events").is_some());
        assert!(body.get("saved_tokens").is_some());
        assert!(body.get("saved_usd").is_some());
        assert_eq!(body["trait_adoption_count"], 14);
    }

    #[tokio::test]
    async fn envelope_batch_endpoint_reports_valid_and_invalid_items() {
        let body = json!([valid_envelope(), {"schema_version": 99}]);
        let response = ocla_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocla/v1/envelope/batch")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let results = json_response(response).await;
        assert_eq!(results.as_array().expect("results").len(), 2);
        assert_eq!(results[0]["valid"], true);
        assert_eq!(results[0]["envelope"], json!(valid_envelope()));
        assert_eq!(results[1]["valid"], false);
        assert!(results[1].get("error").is_some());
    }
}
