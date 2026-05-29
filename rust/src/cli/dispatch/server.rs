// Auto-split from the former monolithic dispatch.rs. run() (the command
// match) stays in mod.rs; standalone helpers grouped by concern.

use super::lifecycle::spawn_proxy_if_needed;
use crate::{core, mcp_stdio, tools};
use anyhow::Result;

pub(super) fn run_mcp_server() -> Result<()> {
    use rmcp::ServiceExt;

    std::env::set_var("LEAN_CTX_MCP_SERVER", "1");

    crate::core::startup_guard::crash_loop_backoff(crate::core::startup_guard::MCP_PROCESS_NAME);

    // Concurrency hardening:
    // - Smooths "thundering herd" MCP startups (multiple agent sessions).
    // - Limits Tokio worker/blocking threads to avoid host degradation.
    // - LEAN_CTX_WORKER_THREADS overrides the default for environments
    //   with many concurrent subagents (e.g. parallel review pipelines).
    let startup_lock = crate::core::startup_guard::try_acquire_lock(
        "mcp-startup",
        std::time::Duration::from_secs(3),
        std::time::Duration::from_secs(30),
    );

    let parallelism = std::thread::available_parallelism().map_or(2, std::num::NonZeroUsize::get);
    let worker_threads = resolve_worker_threads(parallelism);
    let max_blocking_threads = (worker_threads * 4).clamp(8, 32);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(max_blocking_threads)
        .enable_all()
        .build()?;

    let server = tools::create_server();
    drop(startup_lock);

    // Auto-start proxy in background so the dashboard gets exact token data.
    spawn_proxy_if_needed();

    rt.block_on(async {
        core::logging::init_mcp_logging();
        core::protocol::set_mcp_context(true);

        tracing::info!(
            "lean-ctx v{} MCP server starting",
            env!("CARGO_PKG_VERSION")
        );

        let transport =
            mcp_stdio::HybridStdioTransport::new_server(tokio::io::stdin(), tokio::io::stdout());
        let server_handle = server.clone();
        let service = match server.serve(transport).await {
            Ok(s) => s,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("expect initialized")
                    || msg.contains("context canceled")
                    || msg.contains("broken pipe")
                {
                    tracing::debug!("Client disconnected before init: {msg}");
                    return Ok(());
                }
                return Err(e.into());
            }
        };
        match service.waiting().await {
            Ok(reason) => {
                tracing::info!("MCP server stopped: {reason:?}");
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("broken pipe")
                    || msg.contains("connection reset")
                    || msg.contains("context canceled")
                {
                    tracing::info!("MCP server: transport closed ({msg})");
                } else {
                    tracing::error!("MCP server error: {msg}");
                }
            }
        }

        server_handle.shutdown().await;

        core::stats::flush();
        core::heatmap::flush();
        core::mode_predictor::ModePredictor::flush();
        core::feedback::FeedbackStore::flush();

        Ok(())
    })
}

pub(super) fn resolve_worker_threads(parallelism: usize) -> usize {
    std::env::var("LEAN_CTX_WORKER_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| parallelism.clamp(1, 4))
}
