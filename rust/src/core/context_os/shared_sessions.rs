use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::RwLock;

use crate::core::project_hash;
use crate::core::session::SessionState;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SharedSessionKey {
    pub project_hash: String,
    pub workspace_id: String,
    pub channel_id: String,
}

impl SharedSessionKey {
    pub fn new(project_root: &str, workspace_id: &str, channel_id: &str) -> Self {
        Self {
            project_hash: project_hash::hash_project_root(project_root),
            workspace_id: normalize_id(workspace_id, "default"),
            channel_id: normalize_id(channel_id, "default"),
        }
    }
}

pub struct SharedSessionStore {
    sessions: Mutex<HashMap<SharedSessionKey, Arc<RwLock<SessionState>>>>,
}

impl Default for SharedSessionStore {
    fn default() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl SharedSessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_load(
        &self,
        project_root: &str,
        workspace_id: &str,
        channel_id: &str,
    ) -> Arc<RwLock<SessionState>> {
        let key = SharedSessionKey::new(project_root, workspace_id, channel_id);

        if let Some(existing) = self.sessions.lock().ok().and_then(|m| m.get(&key).cloned()) {
            return existing;
        }

        let loaded = load_session_from_disk(project_root, &key)
            .or_else(|| SessionState::load_latest_for_project_root(project_root))
            .unwrap_or_default();

        let mut loaded = loaded;
        loaded.project_root = Some(project_root.to_string());

        let arc = Arc::new(RwLock::new(loaded));
        if let Ok(mut m) = self.sessions.lock() {
            m.insert(key, arc.clone());
        }
        arc
    }

    pub fn persist_best_effort(
        &self,
        project_root: &str,
        workspace_id: &str,
        channel_id: &str,
        session: &SessionState,
    ) {
        let key = SharedSessionKey::new(project_root, workspace_id, channel_id);
        let Some(dir) = shared_session_dir(&key) else {
            return;
        };
        let _ = std::fs::create_dir_all(&dir);
        let state_path = dir.join("session.json");
        let tmp = dir.join("session.json.tmp");

        if let Ok(json) = serde_json::to_string_pretty(session) {
            let _ = std::fs::write(&tmp, json);
            let _ = std::fs::rename(&tmp, &state_path);
        }

        // Persist a compaction snapshot alongside the shared session (premium UX).
        let snap = if session.task.is_some() {
            Some(session.build_compaction_snapshot())
        } else {
            None
        };
        if let Some(snapshot) = snap {
            let _ = std::fs::write(dir.join("snapshot.txt"), snapshot);
        }
    }
}

fn normalize_id(s: &str, fallback: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        fallback.to_string()
    } else {
        // Keep IDs URL/header safe.
        t.chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .collect::<String>()
    }
}

fn shared_session_dir(key: &SharedSessionKey) -> Option<PathBuf> {
    let data = crate::core::data_dir::lean_ctx_data_dir().ok()?;
    Some(
        data.join("context-os")
            .join("sessions")
            .join(&key.project_hash)
            .join(&key.workspace_id)
            .join(&key.channel_id),
    )
}

fn load_session_from_disk(project_root: &str, key: &SharedSessionKey) -> Option<SessionState> {
    let dir = shared_session_dir(key)?;
    let state_path = dir.join("session.json");
    let json = std::fs::read_to_string(&state_path).ok()?;
    let mut session: SessionState = serde_json::from_str(&json).ok()?;
    // Safety: enforce project_root from the current server.
    session.project_root = Some(project_root.to_string());
    Some(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_id_filters_weird_chars() {
        assert_eq!(normalize_id("  ", "x"), "x");
        assert_eq!(normalize_id("abc-123_DEF", "x"), "abc-123_DEF");
        assert_eq!(normalize_id("a b$c", "x"), "abc");
    }

    #[test]
    fn key_is_stable() {
        let k1 = SharedSessionKey::new("/tmp/proj", "ws", "ch");
        let k2 = SharedSessionKey::new("/tmp/proj", "ws", "ch");
        assert_eq!(k1, k2);
    }
}
