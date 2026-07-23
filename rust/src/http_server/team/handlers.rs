#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) async fn v1_manifest(State(_state): State<TeamAppState>) -> impl IntoResponse {
    let v = TeamContextEngine::manifest_value();
    (StatusCode::OK, Json(v))
}

pub(super) async fn v1_tools(
    State(_state): State<TeamAppState>,
    Query(q): Query<ToolsQuery>,
) -> impl IntoResponse {
    let v = TeamContextEngine::manifest_value();
    let tools = v
        .get("tools")
        .and_then(|t| t.get("granular"))
        .cloned()
        .unwrap_or(Value::Array(vec![]));

    let all = tools.as_array().cloned().unwrap_or_default();
    let total = all.len();
    let offset = q.offset.unwrap_or(0).min(total);
    let limit = q.limit.unwrap_or(200).min(500);
    let page = all.into_iter().skip(offset).take(limit).collect::<Vec<_>>();

    (
        StatusCode::OK,
        Json(json!({
            "tools": page,
            "total": total,
            "offset": offset,
            "limit": limit,
        })),
    )
}

pub(super) async fn v1_tool_call(
    State(state): State<TeamAppState>,
    Extension(auth): Extension<TeamAuthContext>,
    Extension(ctx): Extension<TeamRequestContext>,
    Json(body): Json<ToolCallBody>,
) -> impl IntoResponse {
    let workspace_id = body
        .workspace_id
        .clone()
        .unwrap_or_else(|| ctx.workspace_id.clone());
    if !state.team.engine.server.roots.contains_key(&workspace_id) {
        let _ = audit_write(
            &state.team.audit,
            &auth.token_id,
            &workspace_id,
            Some(&body.name),
            Some("/v1/tools/call"),
            false,
            Some("unknown_workspace"),
            body.arguments.as_ref(),
        )
        .await;
        return crate::http_server::json_error(
            StatusCode::BAD_REQUEST,
            "unknown_workspace",
            "unknown workspace",
        );
    }

    let mut args = match body.arguments {
        None => Value::Object(Map::new()),
        Some(Value::Object(m)) => Value::Object(m),
        Some(other) => {
            let _ = audit_write(
                &state.team.audit,
                &auth.token_id,
                &workspace_id,
                Some(&body.name),
                Some("/v1/tools/call"),
                false,
                Some("invalid_arguments"),
                Some(&other),
            )
            .await;
            return crate::http_server::json_error(
                StatusCode::BAD_REQUEST,
                "invalid_arguments",
                &format!("tool arguments must be a JSON object (got {other})"),
            );
        }
    };

    if let Value::Object(ref mut m) = args {
        m.insert(
            WORKSPACE_ARG_KEY.to_string(),
            Value::String(workspace_id.clone()),
        );
        if let Some(ch) = body.channel_id.as_deref()
            && !ch.trim().is_empty()
        {
            m.insert(
                CHANNEL_ARG_KEY.to_string(),
                Value::String(ch.trim().to_string()),
            );
        }
        // Auth-derived agent identity (enterprise#28): overwrite unconditionally
        // so a REST client can never impersonate another token's agent.
        m.insert(
            AGENT_ARG_KEY.to_string(),
            Value::String(format!("team:{}", auth.token_id)),
        );
    }

    let required = required_scopes(&body.name, Some(&args));
    // Index-mutating calls (anything requiring the Index scope) reset the
    // hosted-index freshness baseline once they succeed (GL #391).
    let mutates_index = required
        .as_ref()
        .is_some_and(|reqs| reqs.contains(&TeamScope::Index));
    let allowed = match required {
        None => false,
        Some(reqs) => reqs.is_subset(&auth.scopes),
    };
    if !allowed {
        let _ = audit_write(
            &state.team.audit,
            &auth.token_id,
            &workspace_id,
            Some(&body.name),
            Some("/v1/tools/call"),
            false,
            Some("scope_denied"),
            Some(&args),
        )
        .await;
        return crate::http_server::json_error(
            StatusCode::FORBIDDEN,
            "scope_denied",
            "token lacks required scope for this tool",
        );
    }

    let tool_name = body.name.clone();
    let call = tokio::time::timeout(
        state.timeout,
        state
            .team
            .engine
            .call_tool_value(&tool_name, Some(args.clone())),
    )
    .await;

    match call {
        Ok(Ok(v)) => {
            if mutates_index {
                crate::core::team_slo::global().record_index_write();
            }
            let _ = audit_write(
                &state.team.audit,
                &auth.token_id,
                &workspace_id,
                Some(&tool_name),
                Some("/v1/tools/call"),
                true,
                None,
                Some(&args),
            )
            .await;
            (StatusCode::OK, Json(json!({ "result": v }))).into_response()
        }
        Ok(Err(e)) => {
            let _ = audit_write(
                &state.team.audit,
                &auth.token_id,
                &workspace_id,
                Some(&tool_name),
                Some("/v1/tools/call"),
                true,
                Some("tool_error"),
                Some(&args),
            )
            .await;
            {
                tracing::warn!("team tool call error: {e}");
                crate::http_server::json_error(
                    StatusCode::BAD_REQUEST,
                    "tool_error",
                    "tool execution failed",
                )
            }
        }
        Err(_) => {
            let _ = audit_write(
                &state.team.audit,
                &auth.token_id,
                &workspace_id,
                Some(&tool_name),
                Some("/v1/tools/call"),
                true,
                Some("request_timeout"),
                Some(&args),
            )
            .await;
            crate::http_server::json_error(
                StatusCode::GATEWAY_TIMEOUT,
                "request_timeout",
                "tool call timed out",
            )
        }
    }
}

pub(super) async fn v1_events(
    State(state): State<TeamAppState>,
    Extension(auth): Extension<TeamAuthContext>,
    Extension(ctx): Extension<TeamRequestContext>,
    Query(q): Query<EventsQuery>,
) -> Sse<impl Stream<Item = Result<SseEvent, std::convert::Infallible>>> {
    let ws = ctx.workspace_id;
    let ch = q.channel_id.unwrap_or_else(|| "default".to_string());
    let since = q.since.unwrap_or(0);
    let limit = q.limit.unwrap_or(200).min(1000);

    let _ = audit_event(
        &state.team.audit,
        &auth.token_id,
        &ws,
        &ch,
        "sse_subscribe",
        None,
        since,
    )
    .await;

    let rt = crate::core::context_os::runtime();
    let replay = rt.bus.read(&ws, &ch, since, limit);
    let rx = if let Some(rx) = rt.bus.subscribe(&ws, &ch) {
        rx
    } else {
        tracing::warn!("SSE subscriber limit reached for {ws}/{ch}");
        let (_, rx) = tokio::sync::broadcast::channel::<crate::core::context_os::ContextEventV1>(1);
        rx
    };
    rt.metrics.record_sse_connect();
    rt.metrics.record_events_replayed(replay.len() as u64);
    rt.metrics.record_workspace_active(&ws);

    let bus = rt.bus.clone();
    let metrics = rt.metrics.clone();
    let pending: std::collections::VecDeque<crate::core::context_os::ContextEventV1> =
        replay.into();

    use crate::core::context_os::{RedactionLevel, redact_event_payload};
    let redaction = RedactionLevel::RefsOnly;

    let stream = futures::stream::unfold(
        (
            pending,
            rx,
            ws.clone(),
            ch.clone(),
            since,
            redaction,
            bus,
            metrics,
        ),
        |(mut pending, mut rx, ws, ch, mut last_id, redaction, bus, metrics)| async move {
            if let Some(mut ev) = pending.pop_front() {
                last_id = ev.id;
                redact_event_payload(&mut ev, redaction);
                let data = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_string());
                let evt = SseEvent::default()
                    .id(ev.id.to_string())
                    .event(ev.kind)
                    .data(data);
                return Some((
                    Ok(evt),
                    (pending, rx, ws, ch, last_id, redaction, bus, metrics),
                ));
            }

            loop {
                match rx.recv().await {
                    Ok(mut ev) if ev.id > last_id => {
                        last_id = ev.id;
                        redact_event_payload(&mut ev, redaction);
                        let data = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_string());
                        let evt = SseEvent::default()
                            .id(ev.id.to_string())
                            .event(ev.kind)
                            .data(data);
                        return Some((
                            Ok(evt),
                            (pending, rx, ws, ch, last_id, redaction, bus, metrics),
                        ));
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Closed) => return None,
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let missed = bus.read(&ws, &ch, last_id, skipped as usize);
                        metrics.record_events_replayed(missed.len() as u64);
                        for ev in missed {
                            last_id = last_id.max(ev.id);
                            pending.push_back(ev);
                        }
                    }
                }
            }
        },
    );

    let metrics_ref = rt.metrics.clone();
    let guarded = crate::http_server::SseDisconnectGuard {
        inner: Box::pin(stream),
        metrics: metrics_ref,
    };

    Sse::new(guarded).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

#[derive(Debug, Deserialize)]
pub(super) struct MetricsQuery {
    /// `?format=prometheus` switches to text exposition for scrape agents
    /// (Datadog openmetrics check, Prometheus, Grafana Alloy …).
    #[serde(default)]
    format: Option<String>,
}

pub(super) async fn v1_team_metrics(
    State(_state): State<TeamAppState>,
    Query(q): Query<MetricsQuery>,
) -> Response {
    let slo = crate::core::team_slo::global().snapshot();

    if q.format.as_deref() == Some("prometheus") {
        return (
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; version=0.0.4",
            )],
            slo.to_prometheus(),
        )
            .into_response();
    }

    let rt = crate::core::context_os::runtime();
    let snap = rt.metrics.snapshot();
    let mut v = serde_json::to_value(snap).unwrap_or_default();
    if let Value::Object(ref mut m) = v {
        m.insert(
            "slo".to_string(),
            serde_json::to_value(&slo).unwrap_or_default(),
        );
    }
    (StatusCode::OK, Json(v)).into_response()
}

pub(super) fn streamable_http_config(
    cfg: &TeamServerConfig,
) -> rmcp::transport::StreamableHttpServerConfig {
    let mut out = rmcp::transport::StreamableHttpServerConfig::default()
        .with_stateful_mode(cfg.stateful_mode)
        .with_json_response(cfg.json_response);

    if cfg.disable_host_check {
        out = out.disable_allowed_hosts();
        return out;
    }
    if !cfg.allowed_hosts.is_empty() {
        out = out.with_allowed_hosts(cfg.allowed_hosts.clone());
        return out;
    }
    let host = cfg.host.trim();
    if host == "127.0.0.1" || host == "localhost" || host == "::1" {
        out.allowed_hosts.push(host.to_string());
    }
    out
}
