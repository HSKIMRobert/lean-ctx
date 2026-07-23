#[allow(clippy::wildcard_imports)]
use super::*;

/// Records latency and server-error outcome of every team API request into
/// the process-global SLO store (GL #391). Runs as the outermost layer so the
/// measured latency matches what a client (or the synthetic probe) observes —
/// auth, rate limiting and the handler itself are all included. `/health` and
/// MCP fallback traffic stay unmeasured: the SLO gate is defined over the
/// `/v1` HTTP surface.
pub(super) async fn team_slo_middleware(req: Request<Body>, next: Next) -> Response {
    let measured = {
        let p = req.uri().path();
        p.starts_with("/v1/") || p.starts_with("/api/v1/")
    };
    let start = std::time::Instant::now();
    let res = next.run(req).await;
    if measured {
        crate::core::team_slo::global().record_request(
            u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
            !res.status().is_server_error(),
        );
    }
    res
}

pub(super) async fn team_rate_limit_middleware(
    State(state): State<TeamAppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if req.uri().path() == "/health" {
        return next.run(req).await;
    }
    if !state.rate.allow().await {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    next.run(req).await
}

pub(super) async fn team_concurrency_middleware(
    State(state): State<TeamAppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if req.uri().path() == "/health" {
        return next.run(req).await;
    }
    let Ok(permit) = state.concurrency.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    let resp = next.run(req).await;
    drop(permit);
    resp
}

pub(super) async fn team_auth_middleware(
    State(state): State<TeamAppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    if req.uri().path() == "/health" {
        return next.run(req).await;
    }

    let Some(h) = req.headers().get(header::AUTHORIZATION) else {
        return crate::http_server::json_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "missing Authorization header",
        );
    };
    let Ok(s) = h.to_str() else {
        return crate::http_server::json_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "malformed Authorization header",
        );
    };
    let Some(token) = s
        .strip_prefix("Bearer ")
        .or_else(|| s.strip_prefix("bearer "))
    else {
        return crate::http_server::json_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Authorization must use the Bearer scheme",
        );
    };

    let token_hash = sha256_hex(token.as_bytes());
    let mut matched: Option<TeamTokenConfig> = None;
    for t in state.team.auth.iter() {
        if crate::http_server::constant_time_eq(token_hash.as_bytes(), t.sha256_hex.as_bytes()) {
            matched = Some(t.clone());
            break;
        }
    }
    let Some(tok) = matched else {
        return crate::http_server::json_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "invalid bearer token",
        );
    };
    let tok_scopes: BTreeSet<TeamScope> = tok.effective_scopes();

    let workspace_id = req
        .headers()
        .get(WORKSPACE_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| state.team.engine.server.default_workspace_id.clone());
    if !state.team.engine.server.roots.contains_key(&workspace_id) {
        return crate::http_server::json_error(
            StatusCode::BAD_REQUEST,
            "unknown_workspace",
            "unknown workspace",
        );
    }
    let workspace_id_for_audit = workspace_id.clone();

    req.extensions_mut().insert(TeamAuthContext {
        token_id: tok.id.clone(),
        scopes: tok_scopes.clone(),
    });
    req.extensions_mut()
        .insert(TeamRequestContext { workspace_id });

    // Endpoint-level authz (non-tool endpoints).
    let path0 = req.uri().path();
    if path0 == "/v1/events" {
        let allow = tok_scopes.contains(&TeamScope::Events);
        let _ = audit_write(
            &state.team.audit,
            &tok.id,
            &workspace_id_for_audit,
            None,
            Some("events"),
            allow,
            if allow { None } else { Some("scope_denied") },
            None,
        )
        .await;
        if !allow {
            return crate::http_server::json_error(
                StatusCode::FORBIDDEN,
                "scope_denied",
                "token lacks required scope: events",
            );
        }
    }

    if path0 == "/v1/metrics" {
        let allow = tok_scopes.contains(&TeamScope::Audit);
        let _ = audit_write(
            &state.team.audit,
            &tok.id,
            &workspace_id_for_audit,
            None,
            Some("metrics"),
            allow,
            if allow { None } else { Some("scope_denied") },
            None,
        )
        .await;
        if !allow {
            return crate::http_server::json_error(
                StatusCode::FORBIDDEN,
                "scope_denied",
                "token lacks required scope: audit",
            );
        }
    }

    // Billing-plane reads (savings roll-up, storage/usage reports) share the
    // audit sensitivity class: owner/admin + the control plane's audit token.
    let audit_gated = match path0 {
        "/v1/savings/summary" => Some("savings_summary"),
        "/v1/storage" => Some("storage"),
        "/v1/usage" => Some("usage"),
        "/v1/connectors" => Some("connectors"),
        p if p.starts_with("/v1/savings/member/") => Some("savings_member"),
        _ => None,
    };
    if let Some(action) = audit_gated {
        let allow = tok_scopes.contains(&TeamScope::Audit);
        let _ = audit_write(
            &state.team.audit,
            &tok.id,
            &workspace_id_for_audit,
            None,
            Some(action),
            allow,
            if allow { None } else { Some("scope_denied") },
            None,
        )
        .await;
        if !allow {
            return crate::http_server::json_error(
                StatusCode::FORBIDDEN,
                "scope_denied",
                "token lacks required scope: audit",
            );
        }
    }

    // Tool-level authz for MCP fallback (tools/call).
    let path = req.uri().path().to_string();
    if path != "/v1/tools/call"
        && path != "/v1/tools"
        && path != "/v1/manifest"
        && path != "/health"
    {
        if req.method() != axum::http::Method::POST {
            return next.run(req).await;
        }

        let (parts, body0) = req.into_parts();
        let Ok(bytes) = body::to_bytes(body0, state.max_body_bytes).await else {
            return crate::http_server::json_error(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "could not read request body",
            );
        };

        let mut allow = false;
        let mut denied_reason: Option<String> = None;
        if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
            if v.is_array() {
                denied_reason = Some("batch_requests_not_supported".to_string());
                let _ = audit_write(
                    &state.team.audit,
                    &tok.id,
                    &workspace_id_for_audit,
                    None,
                    None,
                    false,
                    denied_reason.as_deref(),
                    None,
                )
                .await;
            } else {
                let method = v.get("method").and_then(|m| m.as_str()).unwrap_or("");
                if method.eq_ignore_ascii_case("tools/call") {
                    let tool = v
                        .pointer("/params/name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("");
                    let args = v.pointer("/params/arguments");
                    let req_scopes = required_scopes(tool, args);
                    allow = match req_scopes {
                        None => false,
                        Some(reqs) => reqs.is_subset(&tok_scopes),
                    };
                    if !allow {
                        denied_reason = Some("scope_denied".to_string());
                    }
                    let _ = audit_write(
                        &state.team.audit,
                        &tok.id,
                        &workspace_id_for_audit,
                        Some(tool),
                        Some(method),
                        allow,
                        denied_reason.as_deref(),
                        args,
                    )
                    .await;
                } else {
                    allow = true;
                }
            }
        }

        if !allow {
            return crate::http_server::json_error(
                StatusCode::FORBIDDEN,
                "scope_denied",
                "token lacks required scope for this tool",
            );
        }

        req = Request::from_parts(parts, Body::from(bytes));
    }

    next.run(req).await
}

pub(super) async fn audit_write(
    file: &tokio::sync::Mutex<tokio::fs::File>,
    token_id: &str,
    workspace_id: &str,
    tool: Option<&str>,
    method: Option<&str>,
    allowed: bool,
    denied_reason: Option<&str>,
    args: Option<&Value>,
) -> Result<()> {
    let args_hash = args
        .map(|a| {
            let s = a.to_string();
            let mut hasher = Md5::new();
            hasher.update(s.as_bytes());
            crate::core::agent_identity::hex_encode(&hasher.finalize())
        })
        .unwrap_or_default();

    let ts = chrono::Local::now().to_rfc3339();
    let rec = json!({
        "ts": ts,
        "tokenId": token_id,
        "workspaceId": workspace_id,
        "tool": tool,
        "method": method,
        "allowed": allowed,
        "deniedReason": denied_reason,
        "argumentsMd5": args_hash,
    });

    let mut guard = file.lock().await;
    guard.write_all(rec.to_string().as_bytes()).await?;
    guard.write_all(b"\n").await?;
    guard.flush().await?;
    Ok(())
}

/// Event-level audit entry: records who triggered which Context OS event.
pub(super) async fn audit_event(
    file: &tokio::sync::Mutex<tokio::fs::File>,
    token_id: &str,
    workspace_id: &str,
    channel_id: &str,
    event_kind: &str,
    actor: Option<&str>,
    event_id: i64,
) -> Result<()> {
    let ts = chrono::Local::now().to_rfc3339();
    let rec = json!({
        "ts": ts,
        "type": "context_event",
        "tokenId": token_id,
        "workspaceId": workspace_id,
        "channelId": channel_id,
        "eventKind": event_kind,
        "actor": actor,
        "eventId": event_id,
    });

    let mut guard = file.lock().await;
    guard.write_all(rec.to_string().as_bytes()).await?;
    guard.write_all(b"\n").await?;
    guard.flush().await?;
    Ok(())
}
