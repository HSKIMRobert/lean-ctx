//! ctxpkg registry publisher self-service endpoints (GL #406).

#[allow(clippy::wildcard_imports)]
use super::*;

// ── ctxpkg registry publisher self-service (GL #406) ─────────────────────────
//
// Namespace + publish-token management for the logged-in account. Thin
// status-preserving proxies to the private plane; publish/download themselves
// never touch this edge — they go straight to the registry via ctxpkg.com.

/// Request body for `PUT /api/account/registry/namespace`.
#[derive(Deserialize)]
pub(crate) struct RegistryNamespaceBody {
    namespace: String,
    /// Claim on behalf of an org (GL #524) — requires owner/admin there.
    #[serde(default)]
    org_id: Option<String>,
}

/// `PUT /api/account/registry/namespace` — claim the account's publisher
/// namespace on the ctxpkg registry (permanent in v1).
pub(crate) async fn put_account_registry_namespace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegistryNamespaceBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "PUT",
        format!("/api/billing/registry/{user_id}/namespace"),
        Some(json!({ "namespace": body.namespace, "org_id": body.org_id })),
    )
    .await?;
    finish(status, json)
}

/// `GET /api/account/registry` — publisher profile: namespace + token list
/// (metadata only; plaintext tokens are shown exactly once at mint time).
pub(crate) async fn get_account_registry(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "GET",
        format!("/api/billing/registry/{user_id}"),
        None,
    )
    .await?;
    finish(status, json)
}

/// Request body for `POST /api/account/registry/tokens`.
#[derive(Deserialize, Default)]
pub(crate) struct RegistryTokenBody {
    #[serde(default)]
    label: Option<String>,
    /// `publish` (default) or `read` — read tokens are install-only (GL #524).
    #[serde(default)]
    scope: Option<String>,
}

/// `POST /api/account/registry/tokens` — mint a `ctxp_…` publish token or a
/// `ctxr_…` read-only install token.
pub(crate) async fn post_account_registry_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegistryTokenBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "POST",
        format!("/api/billing/registry/{user_id}/tokens"),
        Some(json!({ "label": body.label, "scope": body.scope })),
    )
    .await?;
    finish(status, json)
}

/// `DELETE /api/account/registry/tokens/{token_id}` — revoke a publish token.
pub(crate) async fn delete_account_registry_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token_id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "DELETE",
        format!("/api/billing/registry/{user_id}/tokens/{token_id}"),
        None,
    )
    .await?;
    finish(status, json)
}

/// Request body for `PUT /api/account/registry/price`.
#[derive(Deserialize)]
pub(crate) struct RegistryPriceBody {
    name: String,
    /// `0` / absent clears the price (the pack becomes free again).
    #[serde(default)]
    price_cents: Option<i32>,
}

/// `PUT /api/account/registry/price` — set or clear a package price on the
/// account's namespace (Paid Packs v0, GL #529).
pub(crate) async fn put_account_registry_price(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegistryPriceBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "PUT",
        format!("/api/billing/registry/{user_id}/price"),
        Some(json!({ "name": body.name, "price_cents": body.price_cents })),
    )
    .await?;
    finish(status, json)
}

/// Request body for `POST /api/account/registry/buy`.
#[derive(Deserialize)]
pub(crate) struct RegistryBuyBody {
    namespace: String,
    name: String,
}

/// `POST /api/account/registry/buy` — start a Stripe Checkout for a paid
/// pack; returns the hosted checkout URL (GL #529).
pub(crate) async fn post_account_registry_buy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegistryBuyBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "POST",
        format!("/api/billing/registry/{user_id}/buy"),
        Some(json!({
            "namespace": body.namespace,
            "name": body.name,
            "email": email,
        })),
    )
    .await?;
    finish(status, json)
}

/// Request body for `POST /api/account/registry/domains`.
#[derive(Deserialize)]
pub(crate) struct RegistryDomainBody {
    domain: String,
}

/// `POST /api/account/registry/domains` — register a domain for Verified
/// Publisher and receive the DNS-TXT challenge (GL #516).
pub(crate) async fn post_account_registry_domain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegistryDomainBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "POST",
        format!("/api/billing/registry/{user_id}/domains"),
        Some(json!({ "domain": body.domain })),
    )
    .await?;
    finish(status, json)
}

/// `POST /api/account/registry/domains/{domain_id}/verify` — trigger the
/// DNS-TXT check; flips the publisher to verified on success.
pub(crate) async fn post_account_registry_domain_verify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(domain_id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "POST",
        format!("/api/billing/registry/{user_id}/domains/{domain_id}/verify"),
        None,
    )
    .await?;
    finish(status, json)
}

/// `DELETE /api/account/registry/domains/{domain_id}` — remove a domain.
pub(crate) async fn delete_account_registry_domain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(domain_id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (user_id, _email) = auth_user(&state, &headers).await?;
    let (status, json) = billing_forward(
        &state.cfg,
        "DELETE",
        format!("/api/billing/registry/{user_id}/domains/{domain_id}"),
        None,
    )
    .await?;
    finish(status, json)
}
