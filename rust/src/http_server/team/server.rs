#[allow(clippy::wildcard_imports)]
use super::*;

pub async fn serve_team(cfg: TeamServerConfig) -> Result<()> {
    cfg.validate_for_serve()?;

    let addr: std::net::SocketAddr = format!("{}:{}", cfg.host, cfg.port)
        .parse()
        .context("invalid host/port")?;

    let team_server = TeamCtxServer {
        default_workspace_id: cfg.default_workspace_id.clone(),
        roots: Arc::new(
            cfg.workspaces
                .iter()
                .map(|w| (w.id.clone(), w.root.to_string_lossy().to_string()))
                .collect(),
        ),
    };
    let engine = Arc::new(TeamContextEngine::new(team_server.clone()));

    let audit_file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&cfg.audit_log_path)
        .await
        .with_context(|| format!("open audit log {}", cfg.audit_log_path.display()))?;

    let savings_dir = cfg
        .audit_log_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("savings");
    let workspace_roots: Vec<(String, std::path::PathBuf)> = cfg
        .workspaces
        .iter()
        .map(|w| (w.id.clone(), w.root.clone()))
        .collect();
    let storage_roots = crate::http_server::team_billing::storage_roots_from_config(
        &cfg.audit_log_path,
        &workspace_roots,
        cfg.storage_quota_bytes,
    );
    // Connector run state lives next to the audit log / savings store on the
    // persistent data volume, so per-connector cursors survive redeploys.
    let connectors_state_dir = cfg
        .audit_log_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("connectors");
    let connectors = Arc::new(cfg.connectors.clone());

    // Hosted managed-connector scheduler (#281): scheduled in-process syncs of
    // each configured source into the workspace's BM25/graph/knowledge stores,
    // paused by the storage quota backstop (#282). A no-op with no connectors.
    connectors::spawn_scheduler(
        connectors.clone(),
        team_server.roots.clone(),
        cfg.default_workspace_id.clone(),
        connectors_state_dir.clone(),
        storage_roots.data_root.clone(),
        storage_roots.quota_bytes,
        Duration::from_mins(1),
    );

    let team = Arc::new(TeamState {
        auth: Arc::new(cfg.tokens.clone()),
        engine,
        audit: Arc::new(tokio::sync::Mutex::new(audit_file)),
        savings_store_dir: Arc::new(tokio::sync::Mutex::new(savings_dir)),
        storage_roots,
        storage_cache: Arc::new(tokio::sync::Mutex::new(
            crate::http_server::team_billing::StorageCache::default(),
        )),
        connectors,
        connectors_state_dir: Arc::new(connectors_state_dir),
    });

    let state = TeamAppState {
        concurrency: Arc::new(tokio::sync::Semaphore::new(cfg.max_concurrency.max(1))),
        rate: Arc::new(crate::http_server::RateLimiter::new(
            cfg.max_rps,
            cfg.rate_burst,
        )),
        timeout: Duration::from_millis(cfg.request_timeout_ms.max(1)),
        team,
        max_body_bytes: cfg.max_body_bytes,
    };

    let service_factory =
        move || -> std::result::Result<TeamCtxServer, std::io::Error> { Ok(team_server.clone()) };
    let mcp_http = StreamableHttpService::new(
        service_factory,
        Arc::new(
            rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
        ),
        streamable_http_config(&cfg),
    );

    // Weekly team-ROI webhook (GL #388): validated at boot so a bad URL is a
    // loud startup error, not a silent weekly no-op.
    if let Some(url) = &cfg.roi_webhook_url {
        crate::http_server::roi_webhook::validate_webhook_url(url)
            .map_err(|e| anyhow!("invalid roiWebhookUrl in team config: {e}"))?;
        crate::http_server::roi_webhook::spawn_weekly_roi_webhook(state.clone(), url.clone());
        tracing::info!("team ROI webhook enabled (weekly)");
    }

    let app = Router::new()
        .route("/health", get(crate::http_server::health))
        .route("/v1/manifest", get(v1_manifest))
        .route("/v1/tools", get(v1_tools))
        .route("/v1/tools/call", axum::routing::post(v1_tool_call))
        .route("/v1/events", get(v1_events))
        .route(
            "/v1/context/summary",
            get(crate::http_server::context_views::v1_context_summary),
        )
        .route(
            "/v1/events/search",
            get(crate::http_server::context_views::v1_events_search),
        )
        .route(
            "/v1/events/lineage",
            get(crate::http_server::context_views::v1_event_lineage),
        )
        .route("/v1/metrics", get(v1_team_metrics))
        .route(
            "/v1/savings/summary",
            get(crate::http_server::savings_summary::v1_savings_summary),
        )
        .route(
            "/v1/savings/member/{signer}",
            get(crate::http_server::savings_summary::v1_savings_member),
        )
        .route(
            "/v1/storage",
            get(crate::http_server::team_billing::v1_storage),
        )
        .route("/v1/usage", get(crate::http_server::team_billing::v1_usage))
        .route("/v1/connectors", get(connectors::v1_connectors))
        .route(
            "/api/v1/savings/ingest",
            axum::routing::post(crate::http_server::savings_ingest::v1_savings_ingest),
        )
        .fallback_service(mcp_http)
        .layer(axum::extract::DefaultBodyLimit::max(cfg.max_body_bytes))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            team_rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            team_concurrency_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            team_auth_middleware,
        ))
        // Outermost: SLO measurement sees the full client-observed latency.
        .layer(middleware::from_fn(team_slo_middleware))
        .with_state(state);

    crate::core::team_slo::global().mark_started();

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;

    tracing::info!(
        "lean-ctx TEAM server listening on http://{addr} (workspaces={}, audit={})",
        cfg.workspaces.len(),
        cfg.audit_log_path.display()
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .context("team http server")?;
    Ok(())
}

pub fn create_token() -> Result<(String, String)> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| anyhow!("getrandom: {e}"))?;
    let token = hex_lower(&bytes);
    let hash = sha256_hex(token.as_bytes());
    Ok((token, hash))
}
