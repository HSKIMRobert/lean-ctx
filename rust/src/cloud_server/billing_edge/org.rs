//! Org SSO settings and org audit-log endpoints (GL #482).

#[allow(clippy::wildcard_imports)]
use super::*;

// ── Org SSO settings (GL #482) ────────────────────────────────────────────────

/// Request body for `PUT /api/account/org/sso`.
#[derive(Deserialize)]
pub(crate) struct OrgSsoBody {
    email_domain: String,
    issuer: String,
    client_id: String,
    #[serde(default)]
    client_secret: Option<String>,
}

/// `GET /api/account/org/sso` — the caller's org SSO config (never the secret).
pub(crate) async fn get_account_org_sso(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "GET",
        format!("/api/billing/org/{user_id}/sso"),
        None,
    )
    .await?;
    finish(status, json)
}

/// `PUT /api/account/org/sso` — create/update the org's IdP configuration.
pub(crate) async fn put_account_org_sso(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<OrgSsoBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "PUT",
        format!("/api/billing/org/{user_id}/sso"),
        Some(json!({
            "email_domain": body.email_domain,
            "issuer": body.issuer,
            "client_id": body.client_id,
            "client_secret": body.client_secret,
        })),
    )
    .await?;
    finish(status, json)
}

/// `POST /api/account/org/sso/verify` — run the DNS-TXT domain check.
pub(crate) async fn post_account_org_sso_verify(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "POST",
        format!("/api/billing/org/{user_id}/sso/verify"),
        Some(json!({})),
    )
    .await?;
    finish(status, json)
}

/// Request body for `PUT /api/account/org/sso/required`.
#[derive(Deserialize)]
pub(crate) struct OrgSsoRequiredBody {
    sso_required: bool,
}

/// `PUT /api/account/org/sso/required` — toggle SSO enforcement.
pub(crate) async fn put_account_org_sso_required(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<OrgSsoRequiredBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "PUT",
        format!("/api/billing/org/{user_id}/sso/required"),
        Some(json!({ "sso_required": body.sso_required })),
    )
    .await?;
    finish(status, json)
}

/// `DELETE /api/account/org/sso` — remove the IdP configuration.
pub(crate) async fn delete_account_org_sso(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "DELETE",
        format!("/api/billing/org/{user_id}/sso"),
        None,
    )
    .await?;
    if status == StatusCode::NO_CONTENT {
        return Ok(Json(json!({ "removed": true })));
    }
    finish(status, json)
}

/// Read-side query for the org audit log (GL #484). Mirrors the control-plane
/// contract; all three are optional.
#[derive(Deserialize)]
pub(crate) struct AuditQuery {
    #[serde(default)]
    before: Option<i64>,
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    event: Option<String>,
}

/// Build the sanitized upstream query string. `before`/`limit` are numeric and
/// safe; `event` is allowlisted to our snake_case event ids so nothing
/// untrusted is ever spliced into the upstream URL.
fn build_audit_query(q: &AuditQuery) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(b) = q.before
        && b > 0
    {
        parts.push(format!("before={b}"));
    }
    if let Some(l) = q.limit {
        parts.push(format!("limit={}", l.clamp(1, 200)));
    }
    if let Some(ev) = q.event.as_deref() {
        let ev = ev.trim();
        if !ev.is_empty()
            && ev.len() <= 48
            && ev.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        {
            parts.push(format!("event={ev}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

/// `GET /api/account/org/audit` — the owner's paginated governance audit log.
pub(crate) async fn get_account_org_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let qs = build_audit_query(&q);
    let (status, json) = billing_forward(
        &state.cfg,
        "GET",
        format!("/api/billing/org/{user_id}/audit{qs}"),
        None,
    )
    .await?;
    finish(status, json)
}

/// `GET /api/account/org/audit/export.csv` — the owner's audit log as a CSV
/// download. The control plane renders the CSV; this edge streams it through
/// with the right headers (the body is not JSON, so it bypasses `finish`).
pub(crate) async fn get_account_org_audit_export(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, body) = billing_forward_text(
        &state.cfg,
        format!("/api/billing/org/{user_id}/audit/export.csv"),
    )
    .await?;
    if !status.is_success() {
        return Err((status, "audit export failed".to_string()));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"leanctx-audit-log.csv\"",
            ),
        ],
        body,
    )
        .into_response())
}
