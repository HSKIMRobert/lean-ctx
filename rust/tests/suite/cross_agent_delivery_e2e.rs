//! End-to-end tests for cross-agent delivery via the daemon's OCLA wire API.
//!
//! Simulates two agents (separate Cursor tabs) sharing file reads through the
//! daemon's `/ocla/v1/delivery/{record,check,stats}` endpoints. This validates
//! the complete cross-process pipeline without requiring actual Unix sockets.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

fn ocla_app() -> axum::Router {
    lean_ctx::core::ocla::wire_api::ocla_router()
}

fn json_request(method: &str, uri: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

async fn post_record(app: &axum::Router, agent: &str, path: &str, hash: [u8; 12], mtime: u64) {
    let entry = json!({
        "blake3": hash,
        "path": path,
        "line_count": 150,
        "agent_id": agent,
        "conversation_id": format!("conv-{agent}"),
        "mtime": mtime,
        "token_count": 500,
    });
    let resp = app
        .clone()
        .oneshot(json_request("POST", "/ocla/v1/delivery/record", &entry))
        .await
        .expect("record response");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

async fn post_check(app: &axum::Router, hash: [u8; 12], mtime: u64) -> Value {
    let body = json!({"blake3": hash, "mtime": mtime});
    let resp = app
        .clone()
        .oneshot(json_request("POST", "/ocla/v1/delivery/check", &body))
        .await
        .expect("check response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn get_stats(app: &axum::Router) -> Value {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/ocla/v1/delivery/stats")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("stats response");
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn two_agents_share_reads_via_delivery_endpoints() {
    let app = ocla_app();
    let hash: [u8; 12] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
    let mtime = 1719000000;

    let before = post_check(&app, hash, mtime).await;
    assert_eq!(before["hit"], false, "no prior record → miss");

    post_record(&app, "cursor-tab-1", "src/config.rs", hash, mtime).await;

    let hit = post_check(&app, hash, mtime).await;
    assert_eq!(hit["hit"], true, "agent-2 sees agent-1's read");
    assert_eq!(hit["agent_id"], "cursor-tab-1");
    assert_eq!(hit["path"], "src/config.rs");
    assert_eq!(hit["line_count"], 150);
}

#[tokio::test]
async fn stale_mtime_returns_miss() {
    let app = ocla_app();
    let hash: [u8; 12] = [11, 21, 31, 41, 51, 61, 71, 81, 91, 101, 111, 121];

    post_record(&app, "agent-a", "src/lib.rs", hash, 1000).await;

    let stale = post_check(&app, hash, 2000).await;
    assert_eq!(stale["hit"], false, "different mtime → file was modified");
}

#[tokio::test]
async fn stats_track_cross_agent_activity() {
    let app = ocla_app();
    let hash_a: [u8; 12] = [12, 22, 32, 42, 52, 62, 72, 82, 92, 102, 112, 122];
    let hash_b: [u8; 12] = [13, 23, 33, 43, 53, 63, 73, 83, 93, 103, 113, 123];

    let stats_before = get_stats(&app).await;
    let entries_before = stats_before["total_entries"].as_u64().unwrap();

    post_record(&app, "agent-1", "src/a.rs", hash_a, 500).await;
    post_record(&app, "agent-2", "src/b.rs", hash_b, 600).await;

    let stats_after = get_stats(&app).await;
    let entries_after = stats_after["total_entries"].as_u64().unwrap();
    assert!(
        entries_after >= entries_before + 2,
        "stats must reflect new records"
    );
}

#[tokio::test]
async fn token_savings_accumulate_on_stub_hits() {
    let app = ocla_app();
    let hash: [u8; 12] = [14, 24, 34, 44, 54, 64, 74, 84, 94, 104, 114, 124];

    post_record(&app, "agent-writer", "src/main.rs", hash, 999).await;

    let stats_before = get_stats(&app).await;
    let saved_before = stats_before["tokens_saved"].as_u64().unwrap();

    let _ = post_check(&app, hash, 999).await;
    let _ = post_check(&app, hash, 999).await;

    let stats_after = get_stats(&app).await;
    let saved_after = stats_after["tokens_saved"].as_u64().unwrap();
    assert!(
        saved_after > saved_before,
        "each check hit must accumulate token savings"
    );
    let stubs = stats_after["stubs_served"].as_u64().unwrap();
    assert!(stubs >= 2, "at least 2 stub hits");
}

#[tokio::test]
async fn multiple_agents_multiple_files_isolation() {
    let app = ocla_app();
    let hash_1: [u8; 12] = [15, 25, 35, 45, 55, 65, 75, 85, 95, 105, 115, 125];
    let hash_2: [u8; 12] = [16, 26, 36, 46, 56, 66, 76, 86, 96, 106, 116, 126];

    post_record(&app, "tab-A", "src/models.rs", hash_1, 3000).await;
    post_record(&app, "tab-B", "src/routes.rs", hash_2, 4000).await;

    let check_1 = post_check(&app, hash_1, 3000).await;
    assert_eq!(check_1["hit"], true);
    assert_eq!(check_1["agent_id"], "tab-A");

    let check_2 = post_check(&app, hash_2, 4000).await;
    assert_eq!(check_2["hit"], true);
    assert_eq!(check_2["agent_id"], "tab-B");

    let check_wrong_mtime = post_check(&app, hash_1, 9999).await;
    assert_eq!(check_wrong_mtime["hit"], false);
}
