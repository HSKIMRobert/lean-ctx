use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextEventKindV1 {
    ToolCallRecorded,
    SessionMutated,
    KnowledgeRemembered,
    ArtifactStored,
    GraphBuilt,
    ProofAdded,
}

impl ContextEventKindV1 {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ToolCallRecorded => "tool_call_recorded",
            Self::SessionMutated => "session_mutated",
            Self::KnowledgeRemembered => "knowledge_remembered",
            Self::ArtifactStored => "artifact_stored",
            Self::GraphBuilt => "graph_built",
            Self::ProofAdded => "proof_added",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "session_mutated" => Self::SessionMutated,
            "knowledge_remembered" => Self::KnowledgeRemembered,
            "artifact_stored" => Self::ArtifactStored,
            "graph_built" => Self::GraphBuilt,
            "proof_added" => Self::ProofAdded,
            _ => Self::ToolCallRecorded,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEventV1 {
    pub id: i64,
    pub workspace_id: String,
    pub channel_id: String,
    pub kind: String,
    pub actor: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub payload: Value,
}

#[derive(Clone)]
pub struct ContextBus {
    inner: Arc<Inner>,
}

struct Inner {
    conn: Mutex<Connection>,
    tx: broadcast::Sender<ContextEventV1>,
}

impl Default for ContextBus {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextBus {
    pub fn new() -> Self {
        let path = default_db_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path).expect("open context-os db");
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS context_events (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               workspace_id TEXT NOT NULL,
               channel_id TEXT NOT NULL,
               kind TEXT NOT NULL,
               actor TEXT,
               timestamp TEXT NOT NULL,
               payload_json TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_context_events_stream
               ON context_events(workspace_id, channel_id, id);",
        )
        .expect("init context-os db");

        let (tx, _) = broadcast::channel(1024);
        Self {
            inner: Arc::new(Inner {
                conn: Mutex::new(conn),
                tx,
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ContextEventV1> {
        self.inner.tx.subscribe()
    }

    pub fn append(
        &self,
        workspace_id: &str,
        channel_id: &str,
        kind: &ContextEventKindV1,
        actor: Option<&str>,
        payload: Value,
    ) -> Option<ContextEventV1> {
        let ts = Utc::now();
        let payload_json = payload.to_string();

        let id = {
            let Ok(conn) = self.inner.conn.lock() else {
                return None;
            };
            let _ = conn.execute(
                "INSERT INTO context_events (workspace_id, channel_id, kind, actor, timestamp, payload_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    workspace_id,
                    channel_id,
                    kind.as_str(),
                    actor.map(str::to_string),
                    ts.to_rfc3339(),
                    payload_json
                ],
            );
            conn.last_insert_rowid()
        };

        let ev = ContextEventV1 {
            id,
            workspace_id: workspace_id.to_string(),
            channel_id: channel_id.to_string(),
            kind: kind.as_str().to_string(),
            actor: actor.map(str::to_string),
            timestamp: ts,
            payload,
        };
        let _ = self.inner.tx.send(ev.clone());
        Some(ev)
    }

    pub fn read(
        &self,
        workspace_id: &str,
        channel_id: &str,
        since: i64,
        limit: usize,
    ) -> Vec<ContextEventV1> {
        let limit = limit.clamp(1, 1000) as i64;
        let Ok(conn) = self.inner.conn.lock() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, workspace_id, channel_id, kind, actor, timestamp, payload_json
             FROM context_events
             WHERE workspace_id = ?1 AND channel_id = ?2 AND id > ?3
             ORDER BY id ASC
             LIMIT ?4",
        ) else {
            return Vec::new();
        };
        let rows = stmt
            .query_map(params![workspace_id, channel_id, since, limit], |row| {
                let ts_str: String = row.get(5)?;
                let ts = DateTime::parse_from_rfc3339(&ts_str)
                    .map_or_else(|_| Utc::now(), |d| d.with_timezone(&Utc));
                let payload_str: String = row.get(6)?;
                let payload: Value = serde_json::from_str(&payload_str).unwrap_or(Value::Null);
                Ok(ContextEventV1 {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    channel_id: row.get(2)?,
                    kind: row.get(3)?,
                    actor: row.get::<_, Option<String>>(4)?,
                    timestamp: ts,
                    payload,
                })
            })
            .ok();
        let Some(rows) = rows else {
            return Vec::new();
        };
        rows.flatten().collect()
    }
}

fn default_db_path() -> PathBuf {
    let data = crate::core::data_dir::lean_ctx_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    data.join("context-os").join("context-os.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_read_roundtrip() {
        let bus = ContextBus::new();
        let ev = bus
            .append(
                "ws",
                "ch",
                &ContextEventKindV1::ToolCallRecorded,
                Some("agent"),
                serde_json::json!({"tool":"ctx_read"}),
            )
            .expect("append");
        let got = bus.read("ws", "ch", ev.id - 1, 10);
        assert!(got.iter().any(|e| e.id == ev.id));
    }
}
