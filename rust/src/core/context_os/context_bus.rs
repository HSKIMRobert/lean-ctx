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

    /// Classifies the consistency requirement for this event kind.
    ///
    /// - `Local`: Agent-local, never shared (tool reads, cache hits).
    /// - `Eventual`: Broadcast via bus, other agents see it "soon" (knowledge, artifacts).
    /// - `Strong`: Critical decisions that require acknowledgment before proceeding.
    pub fn consistency_level(&self) -> ConsistencyLevel {
        match self {
            Self::ToolCallRecorded | Self::GraphBuilt => ConsistencyLevel::Local,
            Self::KnowledgeRemembered | Self::ArtifactStored => ConsistencyLevel::Eventual,
            Self::SessionMutated | Self::ProofAdded => ConsistencyLevel::Strong,
        }
    }
}

/// Consistency requirement for shared context events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsistencyLevel {
    /// Agent-local, authoritative: session task, local cache, current file set.
    Local,
    /// Shared, eventually consistent: knowledge facts, gotchas, artifact refs.
    Eventual,
    /// Shared, strongly consistent: workspace config, critical decisions.
    Strong,
}

impl ConsistencyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Eventual => "eventual",
            Self::Strong => "strong",
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
    pub version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<i64>,
    pub consistency_level: String,
    pub payload: Value,
}

impl ContextEventV1 {
    pub fn consistency(&self) -> ConsistencyLevel {
        ContextEventKindV1::parse(&self.kind).consistency_level()
    }
}

fn event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContextEventV1> {
    let ts_str: String = row.get(5)?;
    let ts = DateTime::parse_from_rfc3339(&ts_str)
        .map_or_else(|_| Utc::now(), |d| d.with_timezone(&Utc));
    let payload_str: String = row.get(6)?;
    let payload: Value = serde_json::from_str(&payload_str).unwrap_or(Value::Null);
    let kind_str: String = row.get(3)?;
    let cl = ContextEventKindV1::parse(&kind_str)
        .consistency_level()
        .as_str()
        .to_string();
    Ok(ContextEventV1 {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        channel_id: row.get(2)?,
        kind: kind_str,
        actor: row.get::<_, Option<String>>(4)?,
        timestamp: ts,
        version: row.get::<_, i64>(7).unwrap_or(0),
        parent_id: row.get::<_, Option<i64>>(8).ok().flatten(),
        consistency_level: cl,
        payload,
    })
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
               payload_json TEXT NOT NULL,
               version INTEGER NOT NULL DEFAULT 0,
               parent_id INTEGER
             );
             CREATE INDEX IF NOT EXISTS idx_context_events_stream
               ON context_events(workspace_id, channel_id, id);",
        )
        .expect("init context-os db");

        // Migration: add version + parent_id to existing tables (idempotent).
        let _ = conn.execute_batch(
            "ALTER TABLE context_events ADD COLUMN version INTEGER NOT NULL DEFAULT 0;",
        );
        let _ = conn.execute_batch("ALTER TABLE context_events ADD COLUMN parent_id INTEGER;");

        // FTS5 virtual table for full-text search over event payloads (idempotent).
        let _ = conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS context_events_fts USING fts5(
               payload_text,
               content=context_events,
               content_rowid=id
             );",
        );

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
        self.append_with_parent(workspace_id, channel_id, kind, actor, payload, None)
    }

    pub fn append_with_parent(
        &self,
        workspace_id: &str,
        channel_id: &str,
        kind: &ContextEventKindV1,
        actor: Option<&str>,
        payload: Value,
        parent_id: Option<i64>,
    ) -> Option<ContextEventV1> {
        let ts = Utc::now();
        let payload_json = payload.to_string();

        let (id, version) = {
            let Ok(conn) = self.inner.conn.lock() else {
                return None;
            };
            let version: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(version), 0) FROM context_events WHERE workspace_id = ?1 AND channel_id = ?2",
                    params![workspace_id, channel_id],
                    |row| row.get(0),
                )
                .unwrap_or(0)
                + 1;
            let _ = conn.execute(
                "INSERT INTO context_events (workspace_id, channel_id, kind, actor, timestamp, payload_json, version, parent_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    workspace_id,
                    channel_id,
                    kind.as_str(),
                    actor.map(str::to_string),
                    ts.to_rfc3339(),
                    payload_json,
                    version,
                    parent_id,
                ],
            );
            let rowid = conn.last_insert_rowid();
            let _ = conn.execute(
                "INSERT INTO context_events_fts(rowid, payload_text) VALUES (?1, ?2)",
                params![rowid, payload_json],
            );
            (rowid, version)
        };

        let ev = ContextEventV1 {
            id,
            workspace_id: workspace_id.to_string(),
            channel_id: channel_id.to_string(),
            consistency_level: kind.consistency_level().as_str().to_string(),
            kind: kind.as_str().to_string(),
            actor: actor.map(str::to_string),
            timestamp: ts,
            version,
            parent_id,
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
            "SELECT id, workspace_id, channel_id, kind, actor, timestamp, payload_json, version, parent_id
             FROM context_events
             WHERE workspace_id = ?1 AND channel_id = ?2 AND id > ?3
             ORDER BY id ASC
             LIMIT ?4",
        ) else {
            return Vec::new();
        };
        let rows = stmt
            .query_map(
                params![workspace_id, channel_id, since, limit],
                event_from_row,
            )
            .ok();
        let Some(rows) = rows else {
            return Vec::new();
        };
        rows.flatten().collect()
    }

    /// Query recent events of a specific kind (for conflict detection).
    pub fn recent_by_kind(
        &self,
        workspace_id: &str,
        channel_id: &str,
        kind: &str,
        limit: usize,
    ) -> Vec<ContextEventV1> {
        let limit = limit.clamp(1, 100) as i64;
        let Ok(conn) = self.inner.conn.lock() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, workspace_id, channel_id, kind, actor, timestamp, payload_json, version, parent_id
             FROM context_events
             WHERE workspace_id = ?1 AND channel_id = ?2 AND kind = ?3
             ORDER BY id DESC
             LIMIT ?4",
        ) else {
            return Vec::new();
        };
        let rows = stmt
            .query_map(
                params![workspace_id, channel_id, kind, limit],
                event_from_row,
            )
            .ok();
        rows.map(|r| r.flatten().collect()).unwrap_or_default()
    }

    /// Full-text search over event payloads via FTS5.
    pub fn search(&self, workspace_id: &str, query: &str, limit: usize) -> Vec<ContextEventV1> {
        let limit = limit.clamp(1, 100) as i64;
        let Ok(conn) = self.inner.conn.lock() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT e.id, e.workspace_id, e.channel_id, e.kind, e.actor, e.timestamp,
                    e.payload_json, e.version, e.parent_id
             FROM context_events e
             JOIN context_events_fts f ON e.id = f.rowid
             WHERE f.payload_text MATCH ?1 AND e.workspace_id = ?2
             ORDER BY f.rank
             LIMIT ?3",
        ) else {
            return Vec::new();
        };
        let rows = stmt
            .query_map(params![query, workspace_id, limit], event_from_row)
            .ok();
        rows.map(|r| r.flatten().collect()).unwrap_or_default()
    }

    /// Trace the causal lineage of an event by following parent_id chains.
    pub fn lineage(&self, event_id: i64, max_depth: usize) -> Vec<ContextEventV1> {
        let max_depth = max_depth.clamp(1, 50);
        let Ok(conn) = self.inner.conn.lock() else {
            return Vec::new();
        };
        let mut chain = Vec::new();
        let mut current_id = Some(event_id);

        for _ in 0..max_depth {
            let Some(id) = current_id else {
                break;
            };
            let ev = conn.query_row(
                "SELECT id, workspace_id, channel_id, kind, actor, timestamp, payload_json, version, parent_id
                 FROM context_events WHERE id = ?1",
                params![id],
                event_from_row,
            );
            match ev {
                Ok(ev) => {
                    current_id = ev.parent_id;
                    chain.push(ev);
                }
                Err(_) => break,
            }
        }
        chain
    }

    /// Returns the highest event id for a workspace/channel pair, or 0 if none.
    pub fn latest_id(&self, workspace_id: &str, channel_id: &str) -> i64 {
        let Ok(conn) = self.inner.conn.lock() else {
            return 0;
        };
        conn.query_row(
            "SELECT COALESCE(MAX(id), 0) FROM context_events WHERE workspace_id = ?1 AND channel_id = ?2",
            params![workspace_id, channel_id],
            |row| row.get(0),
        )
        .unwrap_or(0)
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

    #[test]
    fn multi_client_concurrent_appends_have_deterministic_ordering() {
        let bus = Arc::new(ContextBus::new());
        let n_clients = 5;
        let n_events_per_client = 20;
        let ws = format!("ws-concurrent-{}", std::process::id());
        let ch = format!("ch-concurrent-{}", std::process::id());

        let mut handles = vec![];
        for client_idx in 0..n_clients {
            let bus = Arc::clone(&bus);
            let ws = ws.clone();
            let ch = ch.clone();
            handles.push(std::thread::spawn(move || {
                let agent = format!("agent-{client_idx}");
                for event_idx in 0..n_events_per_client {
                    bus.append(
                        &ws,
                        &ch,
                        &ContextEventKindV1::ToolCallRecorded,
                        Some(&agent),
                        serde_json::json!({"client": client_idx, "seq": event_idx}),
                    );
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let all = bus.read(&ws, &ch, 0, 1000);
        assert_eq!(
            all.len(),
            n_clients * n_events_per_client,
            "all events should be persisted"
        );

        let ids: Vec<i64> = all.iter().map(|e| e.id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "events must be in strictly ascending ID order");

        for win in ids.windows(2) {
            assert!(
                win[1] > win[0],
                "IDs must be strictly monotonic (no gaps from concurrent access)"
            );
        }
    }

    #[test]
    fn workspace_channel_isolation() {
        let bus = ContextBus::new();
        let pid = std::process::id();
        let ws_a = format!("ws-iso-a-{pid}");
        let ws_b = format!("ws-iso-b-{pid}");
        let ws_c = format!("ws-iso-c-{pid}");
        let ch1 = format!("ch-iso-1-{pid}");
        let ch2 = format!("ch-iso-2-{pid}");

        bus.append(
            &ws_a,
            &ch1,
            &ContextEventKindV1::SessionMutated,
            Some("agent-a"),
            serde_json::json!({"ws":"a","ch":"1"}),
        );
        bus.append(
            &ws_a,
            &ch2,
            &ContextEventKindV1::KnowledgeRemembered,
            Some("agent-a"),
            serde_json::json!({"ws":"a","ch":"2"}),
        );
        bus.append(
            &ws_b,
            &ch1,
            &ContextEventKindV1::ArtifactStored,
            Some("agent-b"),
            serde_json::json!({"ws":"b","ch":"1"}),
        );

        let ws_a_ch_1 = bus.read(&ws_a, &ch1, 0, 100);
        assert_eq!(ws_a_ch_1.len(), 1);
        assert_eq!(ws_a_ch_1[0].kind, "session_mutated");

        let ws_a_ch_2 = bus.read(&ws_a, &ch2, 0, 100);
        assert_eq!(ws_a_ch_2.len(), 1);
        assert_eq!(ws_a_ch_2[0].kind, "knowledge_remembered");

        let ws_b_ch_1 = bus.read(&ws_b, &ch1, 0, 100);
        assert_eq!(ws_b_ch_1.len(), 1);
        assert_eq!(ws_b_ch_1[0].kind, "artifact_stored");

        let ws_c_ch_1 = bus.read(&ws_c, &ch1, 0, 100);
        assert!(ws_c_ch_1.is_empty(), "non-existent workspace returns empty");
    }

    #[test]
    fn replay_from_cursor_returns_only_newer_events() {
        let bus = ContextBus::new();
        let pid = std::process::id();
        let ws = &format!("ws-replay-{pid}");
        let ch = &format!("ch-replay-{pid}");

        let ev1 = bus
            .append(
                ws,
                ch,
                &ContextEventKindV1::ToolCallRecorded,
                None,
                serde_json::json!({"seq":1}),
            )
            .unwrap();
        let ev2 = bus
            .append(
                ws,
                ch,
                &ContextEventKindV1::SessionMutated,
                None,
                serde_json::json!({"seq":2}),
            )
            .unwrap();
        let _ev3 = bus
            .append(
                ws,
                ch,
                &ContextEventKindV1::GraphBuilt,
                None,
                serde_json::json!({"seq":3}),
            )
            .unwrap();

        let from_cursor = bus.read(ws, ch, ev2.id, 100);
        assert_eq!(from_cursor.len(), 1, "only events after cursor");
        assert_eq!(from_cursor[0].kind, "graph_built");

        let from_first = bus.read(ws, ch, ev1.id, 100);
        assert_eq!(from_first.len(), 2, "events after first");

        let from_zero = bus.read(ws, ch, 0, 100);
        assert_eq!(from_zero.len(), 3, "all events from zero");
    }

    #[test]
    fn broadcast_subscriber_receives_events() {
        let bus = ContextBus::new();
        let mut rx = bus.subscribe();

        let ev = bus
            .append(
                "ws",
                "ch",
                &ContextEventKindV1::ProofAdded,
                Some("verifier"),
                serde_json::json!({"proof":"hash"}),
            )
            .unwrap();

        let received = rx.try_recv().expect("subscriber should receive event");
        assert_eq!(received.id, ev.id);
        assert_eq!(received.kind, "proof_added");
        assert_eq!(received.actor.as_deref(), Some("verifier"));
    }
}
