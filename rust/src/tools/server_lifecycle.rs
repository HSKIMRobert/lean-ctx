use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::core::cache::SessionCache;
use crate::core::session::SessionState;

use super::autonomy;
use super::server::{LeanCtxServer, SessionMode};
use super::startup::detect_startup_context;

impl Default for LeanCtxServer {
    fn default() -> Self {
        Self::new()
    }
}

impl LeanCtxServer {
    /// Creates a new server with default settings, auto-detecting the project root.
    pub fn new() -> Self {
        Self::new_with_project_root(None)
    }

    /// Creates a new server rooted at the given project directory.
    pub fn new_with_project_root(project_root: Option<&str>) -> Self {
        Self::new_with_startup(
            project_root,
            std::env::current_dir().ok().as_deref(),
            SessionMode::Personal,
            "default",
            "default",
        )
    }

    /// Creates a new server in Context OS shared mode for a specific workspace/channel.
    pub fn new_shared_with_context(
        project_root: &str,
        workspace_id: &str,
        channel_id: &str,
    ) -> Self {
        Self::new_with_startup(
            Some(project_root),
            std::env::current_dir().ok().as_deref(),
            SessionMode::Shared,
            workspace_id,
            channel_id,
        )
    }

    pub(crate) fn new_with_startup(
        project_root: Option<&str>,
        startup_cwd: Option<&Path>,
        session_mode: SessionMode,
        workspace_id: &str,
        channel_id: &str,
    ) -> Self {
        let ttl = std::env::var("LEAN_CTX_CACHE_TTL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| {
                let cfg = crate::core::config::Config::load();
                crate::core::config::MemoryCleanup::effective(&cfg).idle_ttl_secs()
            });

        let startup = detect_startup_context(project_root, startup_cwd);
        let (session, context_os) = match session_mode {
            SessionMode::Personal => {
                let mut session = if let Some(ref root) = startup.project_root {
                    SessionState::load_latest_for_project_root(root).unwrap_or_default()
                } else {
                    SessionState::load_latest().unwrap_or_default()
                };
                if let Some(ref root) = startup.project_root {
                    session.project_root = Some(root.clone());
                }
                if let Some(ref cwd) = startup.shell_cwd {
                    session.shell_cwd = Some(cwd.clone());
                }
                (Arc::new(RwLock::new(session)), None)
            }
            SessionMode::Shared => {
                let Some(ref root) = startup.project_root else {
                    // Shared mode without a project root is not useful; fall back to personal.
                    return Self::new_with_startup(
                        project_root,
                        startup_cwd,
                        SessionMode::Personal,
                        workspace_id,
                        channel_id,
                    );
                };
                let rt = crate::core::context_os::runtime();
                let session = rt
                    .shared_sessions
                    .get_or_load(root, workspace_id, channel_id);
                rt.metrics.record_session_loaded();
                // Ensure shell_cwd is refreshed (best-effort).
                if let Some(ref cwd) = startup.shell_cwd {
                    if let Ok(mut s) = session.try_write() {
                        s.shell_cwd = Some(cwd.clone());
                    }
                }
                (session, Some(rt))
            }
        };

        Self {
            cache: Arc::new(RwLock::new(SessionCache::new())),
            session,
            tool_calls: Arc::new(RwLock::new(Vec::new())),
            call_count: Arc::new(AtomicUsize::new(0)),
            cache_ttl_secs: ttl,
            last_call: Arc::new(RwLock::new(Instant::now())),
            agent_id: Arc::new(RwLock::new(None)),
            client_name: Arc::new(RwLock::new(String::new())),
            autonomy: Arc::new(autonomy::AutonomyState::new()),
            loop_detector: Arc::new(RwLock::new(
                crate::core::loop_detection::LoopDetector::with_config(
                    &crate::core::config::Config::load().loop_detection,
                ),
            )),
            workflow: Arc::new(RwLock::new(
                crate::core::workflow::load_active().ok().flatten(),
            )),
            ledger: Arc::new(RwLock::new(
                crate::core::context_ledger::ContextLedger::new(),
            )),
            pipeline_stats: Arc::new(RwLock::new(crate::core::pipeline::PipelineStats::new())),
            session_mode,
            workspace_id: if workspace_id.trim().is_empty() {
                "default".to_string()
            } else {
                workspace_id.trim().to_string()
            },
            channel_id: if channel_id.trim().is_empty() {
                "default".to_string()
            } else {
                channel_id.trim().to_string()
            },
            context_os,
            context_ir: None,
            registry: Some(std::sync::Arc::new(
                crate::server::registry::build_registry(),
            )),
            rules_stale_checked: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            last_seen_event_id: Arc::new(std::sync::atomic::AtomicI64::new(0)),
            startup_project_root: startup.project_root,
            startup_shell_cwd: startup.shell_cwd,
        }
    }

    /// Clears the cache and saves the session if the TTL idle threshold has been exceeded.
    pub async fn check_idle_expiry(&self) {
        if self.cache_ttl_secs == 0 {
            return;
        }
        let last = *self.last_call.read().await;
        if last.elapsed().as_secs() >= self.cache_ttl_secs {
            {
                let mut session = self.session.write().await;
                let _ = session.save();
            }
            let mut cache = self.cache.write().await;
            let count = cache.clear();
            if count > 0 {
                tracing::info!(
                    "Cache auto-cleared after {}s idle ({count} file(s))",
                    self.cache_ttl_secs
                );
            }
        }
        *self.last_call.write().await = Instant::now();
    }

    /// Aggressive cleanup on connection drop: save session, clear all caches, purge allocator.
    pub async fn shutdown(&self) {
        {
            let mut session = self.session.write().await;
            let _ = session.save();
        }
        {
            let mut cache = self.cache.write().await;
            let count = cache.clear();
            if count > 0 {
                tracing::info!("[shutdown] cleared {count} cached file(s)");
            }
        }
        crate::core::memory_guard::force_purge();
    }
}
