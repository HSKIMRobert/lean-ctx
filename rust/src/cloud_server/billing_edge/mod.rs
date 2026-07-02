//! Edge client to the private commercial control-plane (`lean-ctx-cloud`).
//!
//! This is the *only* place the open community backend learns an account's paid
//! plan. It calls the private billing service's `/api/billing/entitlements`
//! endpoint with the shared internal key. If the billing service is not
//! configured or unreachable, every account resolves to
//! [`Plan::Free`](crate::core::billing::Plan) — so the open backend runs fully
//! standalone and **no local capability is ever gated** (Local-Free Invariant).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::core::billing::Plan;

use super::auth::{AppState, auth_user};
use super::config::Config;

/// Resolve a user's effective plan via the private billing service. Any failure
/// (unconfigured, network error, bad response) degrades gracefully to
/// [`Plan::Free`] — the safe default that grants no commercial entitlements.
pub(super) async fn resolve_plan(cfg: &Config, user_id: Uuid) -> Plan {
    resolve_entitlements_raw(cfg, user_id)
        .await
        .and_then(|v| v.get("plan").and_then(Value::as_str).map(Plan::parse))
        .unwrap_or(Plan::Free)
}

// ── Entitlements cache (GL #785) ──────────────────────────────────────────────
//
// A per-user, in-memory cache of the billing plane's entitlements payload. It
// exists for one reason: a brief billing-service outage must never downgrade a
// *paying* account. Without it, `resolve_entitlements_raw` degrades to `Free` on
// any failure, so a single blip would 402 paying Pro users on every
// `/api/sync/*` request (fail-closed against people who pay us).
//
// Policy (mirrors the supporters-wall cache below):
// - fresh within `ENTITLEMENTS_CACHE_TTL` ⇒ serve cached (also shields the plane
//   from per-request traffic),
// - otherwise refetch; on success refresh the slot,
// - on upstream failure ⇒ serve the last value regardless of age (a stale plan
//   beats a wrong downgrade). Only an account never seen before falls through to
//   `Free`, exactly as it did before this cache existed.
//
// Memory is bounded: once the map passes `ENTITLEMENTS_CACHE_MAX`, entries older
// than `ENTITLEMENTS_STALE_RETAIN` are pruned — they could only ever serve as a
// very old stale fallback.

/// How long a fetched entitlements payload counts as fresh.
const ENTITLEMENTS_CACHE_TTL: Duration = Duration::from_mins(1);
/// Soft cap on distinct cached accounts before pruning kicks in.
const ENTITLEMENTS_CACHE_MAX: usize = 50_000;
/// On overflow, evict entries older than this (kept only as stale fallback).
const ENTITLEMENTS_STALE_RETAIN: Duration = Duration::from_hours(1);

struct CachedEntitlements {
    at: Instant,
    value: Value,
}

type EntitlementsCacheSlot = Mutex<HashMap<Uuid, CachedEntitlements>>;
static ENTITLEMENTS_CACHE: LazyLock<EntitlementsCacheSlot> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The cached payload for `user_id` if it was stored less than
/// `ENTITLEMENTS_CACHE_TTL` before `now`. `now` is injected so expiry is
/// unit-testable without sleeping.
fn entitlements_cache_fresh(
    slot: &EntitlementsCacheSlot,
    user_id: Uuid,
    now: Instant,
) -> Option<Value> {
    let guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    guard
        .get(&user_id)
        .filter(|e| now.duration_since(e.at) < ENTITLEMENTS_CACHE_TTL)
        .map(|e| e.value.clone())
}

/// The last cached payload for `user_id` regardless of age — the stale fallback
/// served when the billing plane is unreachable (a stale plan beats a wrong
/// downgrade to Free for a paying account).
fn entitlements_cache_any(slot: &EntitlementsCacheSlot, user_id: Uuid) -> Option<Value> {
    let guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    guard.get(&user_id).map(|e| e.value.clone())
}

/// Store a freshly fetched payload, restarting the TTL window at `now` and
/// pruning very old entries if the map has grown past its soft cap.
fn entitlements_cache_store(
    slot: &EntitlementsCacheSlot,
    user_id: Uuid,
    now: Instant,
    value: &Value,
) {
    let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    guard.insert(
        user_id,
        CachedEntitlements {
            at: now,
            value: value.clone(),
        },
    );
    if guard.len() > ENTITLEMENTS_CACHE_MAX {
        prune_entitlements_cache(&mut guard, now);
    }
}

/// Drop entries older than `ENTITLEMENTS_STALE_RETAIN` relative to `now`. They
/// could only ever serve as a very old stale fallback, so evicting them bounds
/// memory without affecting fresh hits or recent stale fallbacks.
fn prune_entitlements_cache(map: &mut HashMap<Uuid, CachedEntitlements>, now: Instant) {
    map.retain(|_, e| now.duration_since(e.at) < ENTITLEMENTS_STALE_RETAIN);
}

/// The raw entitlements payload from the private billing service (plan,
/// entitlements, org membership — GL #468). Cached per user with a stale-on-error
/// fallback (GL #785), so a billing blip never downgrades a paying account.
/// `None` only when billing is unconfigured, or the account has never been seen
/// and the plane is currently unreachable — callers then degrade to
/// [`Plan::Free`] exactly like before.
async fn resolve_entitlements_raw(cfg: &Config, user_id: Uuid) -> Option<Value> {
    let (Some(base), Some(key)) = (
        cfg.billing_base_url.clone(),
        cfg.billing_internal_key.clone(),
    ) else {
        return None;
    };

    let now = Instant::now();
    if let Some(cached) = entitlements_cache_fresh(&ENTITLEMENTS_CACHE, user_id, now) {
        return Some(cached);
    }

    let url = format!("{base}/api/billing/entitlements/{user_id}");
    let fetched = tokio::task::spawn_blocking(move || {
        ureq::get(&url)
            .header("X-Internal-Key", &key)
            .call()
            .ok()?
            .into_body()
            .read_to_string()
            .ok()
    })
    .await
    .ok()
    .flatten()
    .and_then(|body| serde_json::from_str::<Value>(&body).ok());

    match fetched {
        Some(value) => {
            entitlements_cache_store(&ENTITLEMENTS_CACHE, user_id, now, &value);
            Some(value)
        }
        // Billing unreachable / bad response: serve the last known plan so a blip
        // never downgrades a paying account. Never-seen accounts fall to Free.
        None => entitlements_cache_any(&ENTITLEMENTS_CACHE, user_id),
    }
}

/// Billing-side account deletion (GL #535): cancels any live subscription
/// immediately and purges the user's billing rows.
///
/// Tri-state by design:
/// - billing not configured → `Ok(None)` (nothing to delete — standalone deploy)
/// - billing reachable + 2xx → `Ok(Some(response))`
/// - anything else → `Err(502)` — the caller MUST abort the account deletion,
///   otherwise a paid subscription could keep charging a deleted account.
pub(super) async fn billing_delete_account(
    cfg: &Config,
    user_id: Uuid,
) -> Result<Option<Value>, (StatusCode, String)> {
    let (Some(base), Some(key)) = (
        cfg.billing_base_url.clone(),
        cfg.billing_internal_key.clone(),
    ) else {
        return Ok(None);
    };

    let url = format!("{base}/api/billing/account/{user_id}");
    let body = tokio::task::spawn_blocking(move || {
        ureq::delete(&url)
            .header("X-Internal-Key", &key)
            .call()
            .map_err(|e| e.to_string())?
            .into_body()
            .read_to_string()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("billing deletion failed — account NOT deleted, please retry: {e}"),
        )
    })?;

    Ok(Some(serde_json::from_str(&body).unwrap_or(Value::Null)))
}

/// Whether this deployment leaves cloud sync **ungated** for everyone: either no
/// commercial plane is wired (`billing_base_url` unset), or the operator
/// explicitly opted out (`LEANCTX_CLOUD_SYNC_OPEN=1`). leanctx.com has neither,
/// so sync there is gated to the `cloud_sync` entitlement.
fn sync_is_open(cfg: &Config) -> bool {
    cfg.sync_open || cfg.billing_base_url.is_none()
}

/// Pure cloud-sync gate policy, factored out so it is unit-testable without a
/// DB/billing round-trip. Sync is allowed when the deployment does not gate it
/// at all, or when the caller's plan grants the `cloud_sync` entitlement
/// (Pro/Team/Enterprise). Free and Supporter are denied on a gated deployment.
pub(super) fn cloud_sync_allowed(cfg: &Config, plan: Plan) -> bool {
    sync_is_open(cfg) || plan.entitlements().cloud_sync
}

/// Authenticate the caller **and** require the `cloud_sync` entitlement before a
/// `/api/sync/*` handler proceeds. Returns the same `(user_id, email)` tuple as
/// [`auth_user`], so a call site is a drop-in swap.
///
/// Gating only applies where a commercial plane is actually wired:
/// - **No billing configured** (`billing_base_url` unset) ⇒ open. The community
///   backend runs standalone and sync stays fully usable — nothing is gated
///   without an explicit paid plane (Local-Free Invariant).
/// - **`LEANCTX_CLOUD_SYNC_OPEN=1`** ⇒ open. Operator opt-out for self-hosters
///   who run billing for other reasons but want sync free for everyone.
/// - **Otherwise** ⇒ the account must resolve to a plan whose entitlements grant
///   `cloud_sync` (Pro/Team/Enterprise). Free/Supporter get `402 Payment Required`.
pub(super) async fn require_cloud_sync(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(Uuid, String), (StatusCode, String)> {
    let (user_id, email) = auth_user(state, headers).await?;

    // Resolve the paid plan only where sync is actually gated; open deployments
    // short-circuit without a billing round-trip. `Plan::Free` is a safe stand-in
    // there — `cloud_sync_allowed` returns `true` via its open checks regardless.
    let plan = if sync_is_open(&state.cfg) {
        Plan::Free
    } else {
        resolve_plan(&state.cfg, user_id).await
    };

    if cloud_sync_allowed(&state.cfg, plan) {
        return Ok((user_id, email));
    }

    Err((
        StatusCode::PAYMENT_REQUIRED,
        format!(
            "cloud sync requires lean-ctx Pro (current plan: {}). \
             Run `lean-ctx upgrade` to enable hosted cross-device sync.",
            plan.as_str()
        ),
    ))
}

/// The account's hosted-index quota in MB (GL #392). Paid plans use their
/// `hosted_index_mb` entitlement (Pro: 1000). Open deployments — no billing
/// plane wired, or sync explicitly opened — get a 1000 MB default so the
/// feature works standalone without ever paying (Local-Free Invariant: the
/// hosted bucket is additive, the local index is never gated).
pub(super) async fn hosted_index_quota_mb(state: &AppState, user_id: Uuid) -> u32 {
    if !sync_is_open(&state.cfg) {
        let quota = resolve_plan(&state.cfg, user_id)
            .await
            .entitlements()
            .hosted_index_mb;
        if quota > 0 {
            return quota;
        }
    }
    1_000
}

/// `GET /api/account/entitlements` — the logged-in user's plan, the additive
/// Team/Cloud entitlements it grants, and the org membership (GL #468) the
/// plan may be inherited through (`org: null` for solo accounts).
pub(super) async fn get_account_entitlements(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let raw = resolve_entitlements_raw(&state.cfg, user_id).await;
    let plan = raw
        .as_ref()
        .and_then(|v| v.get("plan").and_then(Value::as_str).map(Plan::parse))
        .unwrap_or(Plan::Free);
    let org = raw
        .as_ref()
        .and_then(|v| v.get("org").cloned())
        .unwrap_or(Value::Null);
    // Subscription lifecycle (GL #535): lets the dashboard show a scheduled
    // cancellation ("ends on July 10 — resume anytime") instead of silence.
    let subscription = raw
        .as_ref()
        .and_then(|v| v.get("subscription").cloned())
        .unwrap_or(Value::Null);
    Ok(Json(json!({
        "plan": plan.as_str(),
        "entitlements": plan.entitlements(),
        "org": org,
        "subscription": subscription,
    })))
}

/// Authenticated server-to-server POST to the private billing service. The shared
/// internal key never leaves the backend. Returns the parsed JSON body, or a
/// `503` when billing is not enabled / `502` when the upstream is unreachable.
async fn billing_post(
    cfg: &Config,
    path: &str,
    payload: Value,
) -> Result<Value, (StatusCode, String)> {
    let (Some(base), Some(key)) = (
        cfg.billing_base_url.clone(),
        cfg.billing_internal_key.clone(),
    ) else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "billing is not enabled on this deployment".to_string(),
        ));
    };

    let url = format!("{base}{path}");
    let bytes = serde_json::to_vec(&payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("encode: {e}")))?;

    let text = tokio::task::spawn_blocking(move || {
        ureq::post(&url)
            .header("X-Internal-Key", &key)
            .header("Content-Type", "application/json")
            .send(&bytes)
            .map_err(|e| e.to_string())?
            .into_body()
            .read_to_string()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| (StatusCode::BAD_GATEWAY, format!("billing upstream: {e}")))?;

    serde_json::from_str::<Value>(&text).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("billing returned non-JSON: {e}"),
        )
    })
}

/// Request body for `POST /api/account/checkout`. `interval` defaults to monthly
/// on the billing side when omitted.
#[derive(Deserialize)]
pub(super) struct CheckoutBody {
    plan: String,
    #[serde(default)]
    interval: Option<String>,
}

/// `POST /api/account/checkout` — start a Stripe Checkout session for the
/// logged-in user and return the hosted `url` to redirect to.
pub(super) async fn post_account_checkout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CheckoutBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, email) = auth_user(&state, &headers).await?;
    let payload = json!({
        "user_id": user_id,
        "email": email,
        "plan": body.plan,
        "interval": body.interval,
    });
    Ok(Json(
        billing_post(&state.cfg, "/api/billing/checkout", payload).await?,
    ))
}

/// `POST /api/account/portal` — open the Stripe billing portal for the logged-in
/// user (manage payment method, invoices, cancel) and return the redirect `url`.
pub(super) async fn post_account_portal(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let payload = json!({ "user_id": user_id });
    Ok(Json(
        billing_post(&state.cfg, "/api/billing/portal", payload).await?,
    ))
}

// ── Hosted Team server dashboard ──────────────────────────────────────────────
//
// Thin, status-preserving proxies to the private plane's team control endpoints.
// The shared internal key never reaches the browser; the caller is identified by
// their session, so the dashboard can only ever act on its own team instance.

/// Forward a team control call to the private plane, preserving the upstream
/// status so the dashboard can surface 404 (no instance yet) / 400 (seat limit)
/// distinctly. Errors only for unset billing (503) or an unreachable plane (502).
async fn billing_forward(
    cfg: &Config,
    method: &'static str,
    path: String,
    payload: Option<Value>,
) -> Result<(StatusCode, Value), (StatusCode, String)> {
    let (Some(base), Some(key)) = (
        cfg.billing_base_url.clone(),
        cfg.billing_internal_key.clone(),
    ) else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "billing is not enabled on this deployment".to_string(),
        ));
    };

    let url = format!("{base}{path}");
    let (code, text) = tokio::task::spawn_blocking(move || -> Result<(u16, String), String> {
        // Read non-2xx as a normal response so the upstream status is preserved.
        let agent: ureq::Agent = ureq::config::Config::builder()
            .tls_config(crate::core::http_client::platform_tls_config())
            .http_status_as_error(false)
            .build()
            .into();
        let resp = match method {
            "GET" => agent.get(&url).header("X-Internal-Key", &key).call(),
            "DELETE" => agent.delete(&url).header("X-Internal-Key", &key).call(),
            // Body methods (POST default, PATCH for partial updates, PUT for
            // full settings replacement). All carry the caller's JSON unchanged.
            _ => {
                let bytes = serde_json::to_vec(&payload.unwrap_or_else(|| json!({})))
                    .map_err(|e| e.to_string())?;
                let builder = match method {
                    "PATCH" => agent.patch(&url),
                    "PUT" => agent.put(&url),
                    _ => agent.post(&url),
                };
                builder
                    .header("X-Internal-Key", &key)
                    .header("Content-Type", "application/json")
                    .send(&bytes)
            }
        }
        .map_err(|e| e.to_string())?;
        let code = resp.status().as_u16();
        let body = resp
            .into_body()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        Ok((code, body))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| (StatusCode::BAD_GATEWAY, format!("billing upstream: {e}")))?;

    let json = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
    let status = StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_GATEWAY);
    Ok((status, json))
}

/// Turn a forwarded `(status, body)` into a handler result, propagating the
/// upstream error message on non-2xx so the dashboard can display it.
fn finish(status: StatusCode, json: Value) -> Result<Json<Value>, (StatusCode, String)> {
    if status.is_success() {
        return Ok(Json(json));
    }
    let msg = json
        .get("error")
        .and_then(Value::as_str)
        .or_else(|| json.get("message").and_then(Value::as_str))
        .unwrap_or("team request failed")
        .to_string();
    Err((status, msg))
}

/// Like [`billing_forward`] but returns the raw upstream body unparsed — used
/// for the CSV export, whose body is `text/csv`, not JSON.
async fn billing_forward_text(
    cfg: &Config,
    path: String,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let (Some(base), Some(key)) = (
        cfg.billing_base_url.clone(),
        cfg.billing_internal_key.clone(),
    ) else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "billing is not enabled on this deployment".to_string(),
        ));
    };
    let url = format!("{base}{path}");
    let (code, text) = tokio::task::spawn_blocking(move || -> Result<(u16, String), String> {
        let agent: ureq::Agent = ureq::config::Config::builder()
            .tls_config(crate::core::http_client::platform_tls_config())
            .http_status_as_error(false)
            .build()
            .into();
        let resp = agent
            .get(&url)
            .header("X-Internal-Key", &key)
            .call()
            .map_err(|e| e.to_string())?;
        let code = resp.status().as_u16();
        let body = resp
            .into_body()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        Ok((code, body))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| (StatusCode::BAD_GATEWAY, format!("billing upstream: {e}")))?;
    let status = StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_GATEWAY);
    Ok((status, text))
}

mod org;
pub(crate) use org::*;
mod registry;
pub(crate) use registry::*;
mod supporters;
pub(crate) use supporters::*;
mod team;
pub(crate) use team::*;
#[cfg(test)]
mod tests;
