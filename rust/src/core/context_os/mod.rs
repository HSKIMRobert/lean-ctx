use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

mod shared_sessions;
pub use shared_sessions::{SharedSessionKey, SharedSessionStore};

mod context_bus;
pub use context_bus::{ContextBus, ContextEventKindV1, ContextEventV1};

/// Shared runtime backing Context OS features (shared sessions + event bus).
///
/// This is intentionally process-local: it enables multi-client coordination
/// for HTTP/daemon/team-server deployments (one process handling many clients).
#[derive(Clone)]
pub struct ContextOsRuntime {
    pub shared_sessions: Arc<SharedSessionStore>,
    pub bus: Arc<ContextBus>,
}

impl Default for ContextOsRuntime {
    fn default() -> Self {
        Self {
            shared_sessions: Arc::new(SharedSessionStore::new()),
            bus: Arc::new(ContextBus::new()),
        }
    }
}

impl ContextOsRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn data_dir() -> Option<PathBuf> {
        crate::core::data_dir::lean_ctx_data_dir().ok()
    }
}

static RUNTIME: OnceLock<Arc<ContextOsRuntime>> = OnceLock::new();

pub fn runtime() -> Arc<ContextOsRuntime> {
    RUNTIME
        .get_or_init(|| Arc::new(ContextOsRuntime::new()))
        .clone()
}
