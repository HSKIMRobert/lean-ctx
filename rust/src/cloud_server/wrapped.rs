//! Hosted opt-in Wrapped permalink (`/api/wrapped`) — the public side of the viral loop.
//!
//! Anonymous publish returns a public `id` + one-time `edit_token`; the token authorizes
//! delete and the optional account `claim`. Only a closed whitelist of aggregate fields is
//! accepted (`deny_unknown_fields`); no repo names, paths, code, history or raw IPs are stored.
//!
//! Contract: `docs/contracts/wrapped-permalink-v1.md`.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};

use super::auth::{auth_user, constant_time_eq, generate_token, sha256_hex, AppState};
use super::helpers::internal_error;

/// Max publishes accepted per `ip_hash` within the rolling rate-limit window.
const MAX_PUBLISH_PER_HOUR: i64 = 20;
const MAX_TOP_COMMANDS: usize = 12;
const MAX_NAME_LEN: usize = 40;
const MAX_LABEL_LEN: usize = 60;

type ApiResult<T> = Result<T, (StatusCode, String)>;

/// JSON error envelope matching the cloud server convention (`helpers::internal_error`).
fn err(status: StatusCode, code: &str) -> (StatusCode, String) {
    (status, format!(r#"{{"error":"{code}"}}"#))
}

fn bad_payload() -> (StatusCode, String) {
    err(StatusCode::BAD_REQUEST, "invalid_payload")
}

// ─── Whitelisted payload (the ONLY fields that may be published) ──────────────

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TopCommand {
    pub name: String,
    pub pct: f64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PublishPayload {
    pub period: String,
    pub tokens_saved: i64,
    pub cost_avoided_usd: f64,
    pub pricing_estimated: bool,
    pub compression_rate_pct: f64,
    pub total_commands: i64,
    pub sessions_count: i64,
    pub files_touched: i64,
    #[serde(default)]
    pub top_commands: Vec<TopCommand>,
    #[serde(default)]
    pub model_key: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

impl PublishPayload {
    /// Rejects anything outside the documented bounds. Pure (no I/O) so it is unit-tested.
    fn validate(&self) -> ApiResult<()> {
        if !matches!(self.period.as_str(), "day" | "week" | "month" | "all") {
            return Err(bad_payload());
        }
        if self.tokens_saved < 0 || self.total_commands < 0 {
            return Err(bad_payload());
        }
        if self.sessions_count < 0 || self.files_touched < 0 {
            return Err(bad_payload());
        }
        if !finite_nonneg(self.cost_avoided_usd) {
            return Err(bad_payload());
        }
        if !in_pct(self.compression_rate_pct) {
            return Err(bad_payload());
        }
        if self.top_commands.len() > MAX_TOP_COMMANDS {
            return Err(bad_payload());
        }
        for c in &self.top_commands {
            let len = c.name.chars().count();
            if len == 0 || len > MAX_NAME_LEN || has_markup(&c.name) || !in_pct(c.pct) {
                return Err(bad_payload());
            }
        }
        if let Some(m) = &self.model_key {
            if m.chars().count() > MAX_LABEL_LEN || has_markup(m) {
                return Err(bad_payload());
            }
        }
        if let Some(name) = &self.display_name {
            let len = name.chars().count();
            if len == 0 || len > MAX_LABEL_LEN || has_markup(name) {
                return Err(bad_payload());
            }
        }
        Ok(())
    }
}

fn finite_nonneg(v: f64) -> bool {
    v.is_finite() && v >= 0.0
}

fn in_pct(v: f64) -> bool {
    v.is_finite() && (0.0..=100.0).contains(&v)
}

/// Rejects markup and control characters — defence against stored XSS in user-chosen text.
fn has_markup(s: &str) -> bool {
    s.chars()
        .any(|c| c == '<' || c == '>' || (c.is_control() && c != '\t'))
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// `POST /api/wrapped` — anonymous publish. Body parsed from raw bytes so unknown/oversized
/// payloads return our own `invalid_payload` / `payload_too_large` instead of axum defaults.
pub(super) async fn publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let payload: PublishPayload = serde_json::from_slice(&body).map_err(|_| bad_payload())?;
    payload.validate()?;

    let client = state.pool.get().await.map_err(internal_error)?;
    let ip_hash = client_ip_hash(&headers, &state.cfg.ip_hash_salt);

    if let Some(h) = &ip_hash {
        let row = client
            .query_one(
                "SELECT count(*) FROM wrapped_cards \
                 WHERE ip_hash = $1 AND created_at > now() - interval '1 hour'",
                &[h],
            )
            .await
            .map_err(internal_error)?;
        let recent: i64 = row.get(0);
        if recent >= MAX_PUBLISH_PER_HOUR {
            return Err(err(StatusCode::TOO_MANY_REQUESTS, "rate_limited"));
        }
    }

    let id = generate_card_id();
    let edit_token = generate_token();
    let edit_token_hash = sha256_hex(&edit_token);
    let payload_json = serde_json::to_string(&payload).map_err(internal_error)?;

    client
        .execute(
            "INSERT INTO wrapped_cards (id, edit_token_hash, payload_json, ip_hash) \
             VALUES ($1, $2, $3, $4)",
            &[&id, &edit_token_hash, &payload_json, &ip_hash],
        )
        .await
        .map_err(internal_error)?;

    let url = format!(
        "{}/w/{}",
        state.cfg.public_base_url.trim_end_matches('/'),
        id
    );
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": id, "edit_token": edit_token, "url": url })),
    ))
}

/// `GET /api/wrapped/:id` — public fetch; increments `view_count` atomically.
pub(super) async fn get_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let client = state.pool.get().await.map_err(internal_error)?;
    let row = client
        .query_opt(
            "UPDATE wrapped_cards SET view_count = view_count + 1 \
             WHERE id = $1 RETURNING payload_json, created_at, view_count",
            &[&id],
        )
        .await
        .map_err(internal_error)?;
    let Some(row) = row else {
        return Err(err(StatusCode::NOT_FOUND, "not_found"));
    };

    let payload_json: String = row.get(0);
    let created_at: chrono::DateTime<chrono::Utc> = row.get(1);
    let view_count: i64 = row.get(2);
    let card: serde_json::Value = serde_json::from_str(&payload_json).map_err(internal_error)?;

    Ok(Json(serde_json::json!({
        "id": id,
        "created_at": created_at.to_rfc3339(),
        "view_count": view_count,
        "card": card,
    })))
}

/// `DELETE /api/wrapped/:id` — requires the matching `X-Edit-Token`.
pub(super) async fn delete_card(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let token =
        edit_token_header(&headers).ok_or_else(|| err(StatusCode::FORBIDDEN, "forbidden"))?;
    let client = state.pool.get().await.map_err(internal_error)?;

    let stored = fetch_token_hash(&client, &id).await?;
    require_token(&token, &stored)?;

    client
        .execute("DELETE FROM wrapped_cards WHERE id = $1", &[&id])
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// `POST /api/wrapped/:id/claim` — binds an anonymous card to the authenticated account.
pub(super) async fn claim_card(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let (user_id, _) = auth_user(&state, &headers).await?;
    let token =
        edit_token_header(&headers).ok_or_else(|| err(StatusCode::FORBIDDEN, "forbidden"))?;
    let client = state.pool.get().await.map_err(internal_error)?;

    let stored = fetch_token_hash(&client, &id).await?;
    require_token(&token, &stored)?;

    client
        .execute(
            "UPDATE wrapped_cards SET user_id = $1 WHERE id = $2",
            &[&user_id, &id],
        )
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::json!({ "claimed": true })))
}

// ─── Helpers ────────────────────────────────────────────────────────────────

async fn fetch_token_hash(client: &tokio_postgres::Client, id: &str) -> ApiResult<String> {
    let row = client
        .query_opt(
            "SELECT edit_token_hash FROM wrapped_cards WHERE id = $1",
            &[&id],
        )
        .await
        .map_err(internal_error)?;
    match row {
        Some(r) => Ok(r.get(0)),
        None => Err(err(StatusCode::NOT_FOUND, "not_found")),
    }
}

fn require_token(presented: &str, stored_hash: &str) -> ApiResult<()> {
    if constant_time_eq(sha256_hex(presented).as_bytes(), stored_hash.as_bytes()) {
        Ok(())
    } else {
        Err(err(StatusCode::FORBIDDEN, "forbidden"))
    }
}

fn edit_token_header(headers: &HeaderMap) -> Option<String> {
    let v = headers.get("x-edit-token")?.to_str().ok()?.trim();
    (!v.is_empty()).then(|| v.to_string())
}

/// 128-bit unguessable, hex-encoded id (the public `/w/<id>` slug).
fn generate_card_id() -> String {
    let bytes: [u8; 16] = rand::random();
    hex::encode(bytes)
}

/// Salted hash of the client IP (from the front proxy's `X-Forwarded-For`/`X-Real-IP`),
/// for abuse rate-limiting only — the raw IP is never stored.
fn client_ip_hash(headers: &HeaderMap, salt: &str) -> Option<String> {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })?;
    Some(sha256_hex(&format!("{salt}:{ip}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> PublishPayload {
        PublishPayload {
            period: "week".into(),
            tokens_saved: 480_600_000,
            cost_avoided_usd: 1441.79,
            pricing_estimated: true,
            compression_rate_pct: 91.2,
            total_commands: 1234,
            sessions_count: 56,
            files_touched: 789,
            top_commands: vec![TopCommand {
                name: "ctx_search".into(),
                pct: 60.0,
            }],
            model_key: Some("claude-opus".into()),
            display_name: Some("yvesg".into()),
        }
    }

    #[test]
    fn accepts_a_well_formed_payload() {
        assert!(valid().validate().is_ok());
    }

    #[test]
    fn rejects_unknown_fields() {
        let json = r#"{"period":"week","tokens_saved":1,"cost_avoided_usd":0.1,
            "pricing_estimated":false,"compression_rate_pct":50,"total_commands":1,
            "sessions_count":1,"files_touched":1,"repo_path":"/secret/path"}"#;
        assert!(serde_json::from_str::<PublishPayload>(json).is_err());
    }

    #[test]
    fn rejects_bad_period_and_ranges() {
        let mut p = valid();
        p.period = "year".into();
        assert!(p.validate().is_err());

        let mut p = valid();
        p.compression_rate_pct = 150.0;
        assert!(p.validate().is_err());

        let mut p = valid();
        p.tokens_saved = -1;
        assert!(p.validate().is_err());

        let mut p = valid();
        p.cost_avoided_usd = f64::NAN;
        assert!(p.validate().is_err());
    }

    #[test]
    fn rejects_oversized_and_markup_text() {
        let mut p = valid();
        p.display_name = Some("a".repeat(MAX_LABEL_LEN + 1));
        assert!(p.validate().is_err());

        let mut p = valid();
        p.display_name = Some("<script>".into());
        assert!(p.validate().is_err());

        let mut p = valid();
        p.top_commands = (0..MAX_TOP_COMMANDS + 1)
            .map(|_| TopCommand {
                name: "git".into(),
                pct: 1.0,
            })
            .collect();
        assert!(p.validate().is_err());
    }

    #[test]
    fn ip_hash_is_salted_and_omitted_without_headers() {
        let mut h = HeaderMap::new();
        assert!(client_ip_hash(&h, "salt").is_none());

        h.insert("x-forwarded-for", "203.0.113.7, 10.0.0.1".parse().unwrap());
        let a = client_ip_hash(&h, "salt-a").unwrap();
        let b = client_ip_hash(&h, "salt-b").unwrap();
        assert_ne!(a, b, "different salts must yield different hashes");
        assert!(!a.contains("203.0.113.7"), "raw IP must never appear");
    }
}
