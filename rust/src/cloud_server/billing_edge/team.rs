//! Team, seats, invites and member-token endpoints (thin proxies to the private plane).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Request body for issuing a team member token.
#[derive(Deserialize)]
pub(crate) struct MemberBody {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    label: Option<String>,
}

/// `GET /api/account/team` — the logged-in owner's hosted team server status and
/// member roster (no secrets). `provisioned:false` until a Team plan deploys one.
pub(crate) async fn get_account_team(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "GET",
        format!("/api/billing/team/{user_id}"),
        None,
    )
    .await?;
    finish(status, json)
}

/// `GET /api/account/team/savings` — the logged-in owner's aggregated team
/// savings roll-up (net tokens + USD saved, per member and per model). Returns
/// `savings_available:false` until the hosted server has received at least one
/// signed batch, or `provisioned:false` when no team server exists yet.
pub(crate) async fn get_account_team_savings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "GET",
        format!("/api/billing/team/{user_id}/savings"),
        None,
    )
    .await?;
    finish(status, json)
}

/// `GET /api/account/team/savings/member/{signer}` — per-member drilldown
/// (GL #389): the signer's own 90-day cumulative series plus model/tool
/// breakdowns. 404 when the signer never reported a batch. The signer id is
/// the truncated public key from `summary.by_member[].signer` (URL-safe by
/// construction; anything else is rejected upstream).
pub(crate) async fn get_account_team_savings_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(signer): axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    // Tight allowlist before the id is embedded in an upstream URL.
    if signer.is_empty()
        || signer.len() > 64
        || !signer
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err((StatusCode::BAD_REQUEST, "invalid signer id".into()));
    }
    let (status, json) = billing_forward(
        &state.cfg,
        "GET",
        format!("/api/billing/team/{user_id}/savings/member/{signer}"),
        None,
    )
    .await?;
    finish(status, json)
}

/// Internal GET against the billing plane for the digest job (GL #386) —
/// no user session involved, the job acts on the server's own behalf.
/// `None` when billing is unconfigured or unreachable.
pub(crate) async fn forward_for_digest(cfg: &Config, path: String) -> Option<(u16, Value)> {
    match billing_forward(cfg, "GET", path, None).await {
        Ok((status, json)) => Some((status.as_u16(), json)),
        Err(_) => None,
    }
}

/// Request body for team settings (GL #388).
#[derive(Deserialize)]
pub(crate) struct TeamSettingsBody {
    #[serde(default, rename = "roiWebhookUrl")]
    roi_webhook_url: Option<String>,
}

/// `PUT /api/account/team/settings` — owner-tunable team-server settings
/// (currently the weekly ROI webhook URL, GL #388). Validation and the
/// config re-render happen in the control plane; this edge only forwards.
pub(crate) async fn put_account_team_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TeamSettingsBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "PUT",
        format!("/api/billing/team/{user_id}/settings"),
        Some(json!({ "roiWebhookUrl": body.roi_webhook_url })),
    )
    .await?;
    finish(status, json)
}

/// `POST /api/account/team/owner-token` — (re)issue the owner token, returned
/// exactly once. Rotates any prior owner credential and redeploys the server.
pub(crate) async fn post_account_team_owner_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "POST",
        format!("/api/billing/team/{user_id}/owner-token"),
        Some(json!({})),
    )
    .await?;
    finish(status, json)
}

/// `POST /api/account/team/members` — issue a seat-limited member token (returned
/// once). 400 from the plane when the plan's seat limit is reached.
pub(crate) async fn post_account_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MemberBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "POST",
        format!("/api/billing/team/{user_id}/tokens"),
        Some(json!({ "role": body.role, "label": body.label })),
    )
    .await?;
    finish(status, json)
}

// ── Invite links (GL #385) ────────────────────────────────────────────────────

/// Request body for `POST /api/account/team/invites`.
#[derive(Deserialize)]
pub(crate) struct InviteBody {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

/// `POST /api/account/team/invites` — mint a one-time invite link for the
/// logged-in owner's team. The code is returned exactly once; the dashboard
/// turns it into `https://leanctx.com/join/?code=…`.
pub(crate) async fn post_account_team_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<InviteBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "POST",
        format!("/api/billing/team/{user_id}/invites"),
        Some(json!({ "label": body.label, "role": body.role })),
    )
    .await?;
    finish(status, json)
}

/// `GET /api/account/team/invites` — the owner's invite audit list
/// (pending / used / revoked / expired; never the codes themselves).
pub(crate) async fn get_account_team_invites(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "GET",
        format!("/api/billing/team/{user_id}/invites"),
        None,
    )
    .await?;
    finish(status, json)
}

/// `DELETE /api/account/team/invites/{invite_id}` — revoke a pending invite.
pub(crate) async fn delete_account_team_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(invite_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "DELETE",
        format!("/api/billing/team/{user_id}/invites/{invite_id}"),
        None,
    )
    .await?;
    if status == StatusCode::NO_CONTENT {
        return Ok(Json(json!({ "revoked": true })));
    }
    finish(status, json)
}

/// Forward an invite redemption to the control plane on behalf of the (login-
/// less) teammate. Used by the public join endpoint (`team_join.rs`).
pub(crate) async fn forward_invite_redeem(
    cfg: &Config,
    code: &str,
) -> Result<(StatusCode, Value), (StatusCode, String)> {
    billing_forward(
        cfg,
        "POST",
        "/api/billing/invites/redeem".to_string(),
        Some(json!({ "code": code })),
    )
    .await
}

/// `DELETE /api/account/team/members/{token_id}` — revoke a member token and
/// redeploy. The owner token cannot be revoked (the plane rejects it).
pub(crate) async fn delete_account_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "DELETE",
        format!("/api/billing/team/{user_id}/tokens/{token_id}"),
        None,
    )
    .await?;
    if status.is_success() {
        return Ok(Json(json!({ "revoked": true })));
    }
    finish(status, json)
}

// ── Team seats, hosted-index storage & managed connectors ─────────────────────
//
// Same thin-proxy pattern as the team roster above: authenticate the owner by
// their session, forward to the private plane with the internal key, and preserve
// the upstream status. Request bodies are passed through unchanged (the plane owns
// validation), so the edge never duplicates the seat/connector schema.

/// `POST /api/account/team/seats` — change the team's seat count (written straight
/// to the Stripe subscription, prorated). Body `{ "seats": N }`; returns the
/// refreshed team payload so the dashboard re-renders in one round-trip.
pub(crate) async fn post_account_team_seats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "POST",
        format!("/api/billing/team/{user_id}/seats"),
        Some(body),
    )
    .await?;
    finish(status, json)
}

/// `GET /api/account/team/storage` — hosted retrieval-index footprint + metering.
/// `available:false` until a team server is provisioned and reports storage.
pub(crate) async fn get_account_team_storage(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "GET",
        format!("/api/billing/team/{user_id}/storage"),
        None,
    )
    .await?;
    finish(status, json)
}

/// `GET /api/account/team/connectors` — the secret-free managed-connector roster,
/// each merged with its latest live sync status.
pub(crate) async fn get_account_team_connectors(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "GET",
        format!("/api/billing/team/{user_id}/connectors"),
        None,
    )
    .await?;
    finish(status, json)
}

/// `POST /api/account/team/connectors` — create a managed connector. The plaintext
/// provider secret is forwarded once to the plane (encrypted at rest there) and is
/// never stored or echoed by the edge. 400 from the plane on validation / limit.
pub(crate) async fn post_account_team_connector(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "POST",
        format!("/api/billing/team/{user_id}/connectors"),
        Some(body),
    )
    .await?;
    finish(status, json)
}

/// `PATCH /api/account/team/connectors/{connector_id}` — pause/resume a connector.
/// Body `{ "enabled": bool }`. The plane returns 204 No Content, so surface a
/// small JSON ack the dashboard can treat as success.
pub(crate) async fn patch_account_team_connector(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(connector_id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "PATCH",
        format!("/api/billing/team/{user_id}/connectors/{connector_id}"),
        Some(body),
    )
    .await?;
    if status.is_success() {
        return Ok(Json(json!({ "updated": true })));
    }
    finish(status, json)
}

/// `DELETE /api/account/team/connectors/{connector_id}` — remove a connector and
/// redeploy. The plane returns 204 No Content; surface a JSON ack.
pub(crate) async fn delete_account_team_connector(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(connector_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "DELETE",
        format!("/api/billing/team/{user_id}/connectors/{connector_id}"),
        None,
    )
    .await?;
    if status.is_success() {
        return Ok(Json(json!({ "deleted": true })));
    }
    finish(status, json)
}
