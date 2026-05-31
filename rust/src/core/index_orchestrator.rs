use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::core::bm25_index::BM25Index;
use crate::core::graph_index::{self, ProjectIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Building,
    Ready,
    Failed,
}

#[derive(Debug, Clone)]
struct Component {
    state: State,
    started_ms: Option<u64>,
    finished_ms: Option<u64>,
    duration_ms: Option<u64>,
    last_error: Option<String>,
    /// Human-readable outcome detail surfaced to operators (e.g. doc count +
    /// persisted size, or the "not persisted: too large …" remedy). Independent
    /// of `last_error` so a *successful* build can still carry a warning note.
    note: Option<String>,
}

impl Component {
    fn new() -> Self {
        Self {
            state: State::Idle,
            started_ms: None,
            finished_ms: None,
            duration_ms: None,
            last_error: None,
            note: None,
        }
    }
}

#[derive(Debug)]
struct ProjectBuild {
    worker_running: bool,
    graph: Component,
    bm25: Component,
}

impl ProjectBuild {
    fn new() -> Self {
        Self {
            worker_running: false,
            graph: Component::new(),
            bm25: Component::new(),
        }
    }
}

// Lock ordering (see rust/LOCK_ORDERING.md):
//   L1 = REGISTRY outer Mutex  (the HashMap guard)
//   L2 = per-project Arc<Mutex<ProjectBuild>>  (inner guard)
//
// Invariant: L1 must NEVER be held while locking L2.
// `entry_for()` enforces this by cloning the Arc and dropping L1 before
// the caller acquires L2.
static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<Mutex<ProjectBuild>>>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, Arc<Mutex<ProjectBuild>>>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn entry_for(project_root: &str) -> Arc<Mutex<ProjectBuild>> {
    let mut map = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    map.entry(project_root.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(ProjectBuild::new())))
        .clone()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn start_component(c: &mut Component) {
    c.state = State::Building;
    c.started_ms = Some(now_ms());
    c.finished_ms = None;
    c.duration_ms = None;
    c.last_error = None;
    c.note = None;
}

fn finish_ok(c: &mut Component) {
    c.state = State::Ready;
    let end = now_ms();
    c.finished_ms = Some(end);
    c.duration_ms = c.started_ms.map(|s| end.saturating_sub(s));
}

fn finish_err(c: &mut Component, e: String) {
    c.state = State::Failed;
    let end = now_ms();
    c.finished_ms = Some(end);
    c.duration_ms = c.started_ms.map(|s| end.saturating_sub(s));
    c.last_error = Some(e);
}

pub fn ensure_all_background(project_root: &str) {
    let state = entry_for(project_root);
    let should_spawn = {
        let mut s = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if s.worker_running {
            false
        } else {
            s.worker_running = true;
            true
        }
    };

    if !should_spawn {
        return;
    }

    let root = project_root.to_string();
    std::thread::spawn(move || {
        let state = entry_for(&root);

        // Pre-warm the resident line-search index in parallel (own thread,
        // deduped internally) so the first ctx_search hits the fast path.
        crate::core::search_index::ensure_background(&root, true, false);

        // Phase 1: Graph index — may produce a content cache from the file walk
        {
            let mut s = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            start_component(&mut s.graph);
        }
        let graph_result = std::panic::catch_unwind(|| {
            let (idx, content_cache) = graph_index::scan_with_content_cache(&root);
            // JSON index write is kept for backward compatibility with remaining
            // direct ProjectIndex consumers. Will be removed when all consumers
            // are migrated to GraphProvider/PropertyGraph. (OPT-14/15 Phase 6)
            let _ = idx.save();
            (idx, content_cache)
        });
        let content_cache = if let Ok((_idx, cache)) = graph_result {
            let mut s = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            finish_ok(&mut s.graph);
            cache
        } else {
            let mut s = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            finish_err(&mut s.graph, "graph index build panicked".to_string());
            HashMap::new()
        };

        // Phase 2: BM25 index — reuses content from graph scan when available
        {
            let mut s = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            start_component(&mut s.bm25);
        }
        let bm = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let root_pb = Path::new(&root);
            let idx = if content_cache.is_empty() {
                BM25Index::load_or_build(root_pb)
            } else {
                BM25Index::build_with_content_hint(root_pb, &content_cache)
            };
            let outcome = idx.save(root_pb);
            (idx.doc_count, outcome)
        }));
        if let Ok((doc_count, save_res)) = bm {
            let mut s = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            finish_ok(&mut s.bm25);
            s.bm25.note = Some(bm25_build_note(doc_count, &save_res));
        } else {
            let mut s = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            finish_err(&mut s.bm25, "bm25 build panicked".to_string());
        }

        let mut s = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        s.worker_running = false;
    });
}

/// Build a human-readable outcome note for a finished BM25 build, including the
/// indexed chunk count and whether the index was persisted to disk. A
/// "too large" refusal carries the exact remedy so the operator (or agent) is
/// never left guessing why search/ranking stays cold (issue #249).
fn bm25_build_note(
    doc_count: usize,
    save: &std::io::Result<crate::core::bm25_index::SaveOutcome>,
) -> String {
    use crate::core::bm25_index::SaveOutcome;
    match save {
        Ok(SaveOutcome::Persisted { compressed_bytes }) => format!(
            "indexed {doc_count} chunks, {:.1} MB persisted",
            *compressed_bytes as f64 / 1_048_576.0
        ),
        Ok(SaveOutcome::SkippedTooLarge {
            compressed_bytes,
            limit_bytes,
        }) => format!(
            "indexed {doc_count} chunks but NOT persisted to disk: compressed {:.1} MB exceeds the {:.0} MB cap. \
             Raise it via LEAN_CTX_BM25_MAX_CACHE_MB (or bm25_max_cache_mb in config) or add extra_ignore_patterns, \
             then run `lean-ctx reindex`. Until then the index is rebuilt from scratch on every cold start.",
            *compressed_bytes as f64 / 1_048_576.0,
            *limit_bytes as f64 / 1_048_576.0
        ),
        Err(e) => format!("indexed {doc_count} chunks but persisting failed: {e}"),
    }
}

/// Lightweight, allocation-frugal snapshot of the BM25 component for the
/// in-call composer/search messaging. Avoids the heavier [`disk_status`] walk.
#[derive(Debug, Clone)]
pub struct Bm25Summary {
    pub state: &'static str,
    /// While building: elapsed so far. Otherwise: last build duration.
    pub elapsed_ms: Option<u64>,
    pub note: Option<String>,
    pub last_error: Option<String>,
}

pub fn bm25_summary(project_root: &str) -> Bm25Summary {
    let entry = entry_for(project_root);
    let s = entry
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let c = &s.bm25;
    let elapsed_ms = if matches!(c.state, State::Building) {
        c.started_ms.map(|start| now_ms().saturating_sub(start))
    } else {
        c.duration_ms
    };
    Bm25Summary {
        state: match c.state {
            State::Idle => "idle",
            State::Building => "building",
            State::Ready => "ready",
            State::Failed => "failed",
        },
        elapsed_ms,
        note: c.note.clone(),
        last_error: c.last_error.clone(),
    }
}

pub fn try_load_graph_index(project_root: &str) -> Option<ProjectIndex> {
    // Resident cache: avoids re-reading + zstd-decompressing + serde-parsing the
    // on-disk index on every graph-touching query. Returns an in-memory clone.
    crate::core::graph_cache::get_cached(project_root).map(|arc| (*arc).clone())
}

pub fn try_load_bm25_index(project_root: &str) -> Option<BM25Index> {
    BM25Index::load(Path::new(project_root))
}

/// Returns true if any project is currently building its indices.
pub fn is_building() -> bool {
    let map = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    map.values().any(|entry| {
        let s = entry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        matches!(s.bm25.state, State::Building) || matches!(s.graph.state, State::Building)
    })
}

#[derive(Debug, Serialize)]
struct ComponentStatus<'a> {
    state: &'a str,
    started_ms: Option<u64>,
    finished_ms: Option<u64>,
    duration_ms: Option<u64>,
    last_error: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<&'a str>,
}

fn component_status(c: &Component) -> ComponentStatus<'_> {
    ComponentStatus {
        state: match c.state {
            State::Idle => "idle",
            State::Building => "building",
            State::Ready => "ready",
            State::Failed => "failed",
        },
        started_ms: c.started_ms,
        finished_ms: c.finished_ms,
        duration_ms: c.duration_ms,
        last_error: c.last_error.as_deref(),
        note: c.note.as_deref(),
    }
}

#[derive(Debug, Serialize)]
struct StatusResponse<'a> {
    project_root: &'a str,
    graph_index: ComponentStatus<'a>,
    bm25_index: ComponentStatus<'a>,
    disk: DiskStatusAll,
}

#[derive(Debug, Serialize, Default)]
pub struct DiskStatus {
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub file_count: Option<u64>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct DiskStatusAll {
    pub graph_index: DiskStatus,
    pub bm25_index: DiskStatus,
    pub code_graph: DiskStatus,
}

fn disk_status_for_graph(project_root: &str) -> DiskStatus {
    let Some(dir) = graph_index::ProjectIndex::index_dir(project_root) else {
        return DiskStatus::default();
    };
    let zst = dir.join("index.json.zst");
    let json = dir.join("index.json");
    let path = if zst.exists() {
        zst
    } else if json.exists() {
        json
    } else {
        return DiskStatus::default();
    };
    let meta = std::fs::metadata(&path).ok();
    let file_count =
        graph_index::ProjectIndex::load(project_root).map(|idx| idx.files.len() as u64);
    DiskStatus {
        exists: true,
        size_bytes: meta.as_ref().map(std::fs::Metadata::len),
        file_count,
        modified_at: meta.and_then(|m| m.modified().ok()).map(format_time),
    }
}

fn disk_status_for_bm25(project_root: &str) -> DiskStatus {
    let root = Path::new(project_root);
    let path = BM25Index::index_file_path(root);
    if !path.exists() {
        return DiskStatus::default();
    }
    let meta = std::fs::metadata(&path).ok();
    DiskStatus {
        exists: true,
        size_bytes: meta.as_ref().map(std::fs::Metadata::len),
        file_count: None,
        modified_at: meta.and_then(|m| m.modified().ok()).map(format_time),
    }
}

fn disk_status_for_code_graph(project_root: &str) -> DiskStatus {
    let dir = crate::core::property_graph::graph_dir(project_root);
    let db_path = dir.join("graph.db");
    if !db_path.exists() {
        return DiskStatus::default();
    }
    let meta = std::fs::metadata(&db_path).ok();
    let node_count = crate::core::property_graph::CodeGraph::open(project_root)
        .ok()
        .and_then(|g| {
            g.connection()
                .query_row("SELECT count(*) FROM nodes", [], |r| r.get::<_, i64>(0))
                .ok()
                .map(|c| c as u64)
        });
    DiskStatus {
        exists: true,
        size_bytes: meta.as_ref().map(std::fs::Metadata::len),
        file_count: node_count,
        modified_at: meta.and_then(|m| m.modified().ok()).map(format_time),
    }
}

fn format_time(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let dt = chrono::DateTime::from_timestamp(secs as i64, 0);
    dt.map_or_else(
        || format!("{secs}"),
        |d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
    )
}

pub fn disk_status(project_root: &str) -> DiskStatusAll {
    DiskStatusAll {
        graph_index: disk_status_for_graph(project_root),
        bm25_index: disk_status_for_bm25(project_root),
        code_graph: disk_status_for_code_graph(project_root),
    }
}

pub fn status_json(project_root: &str) -> String {
    let state = entry_for(project_root);
    let s = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let res = StatusResponse {
        project_root,
        graph_index: component_status(&s.graph),
        bm25_index: component_status(&s.bm25),
        disk: disk_status(project_root),
    };
    serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_json_is_valid_json() {
        let s = status_json("/tmp");
        let _: serde_json::Value = serde_json::from_str(&s).unwrap();
    }

    #[test]
    fn build_note_persisted_reports_size() {
        let note = bm25_build_note(
            42,
            &Ok(crate::core::bm25_index::SaveOutcome::Persisted {
                compressed_bytes: 3 * 1024 * 1024,
            }),
        );
        assert!(
            note.contains("42 chunks"),
            "note should report chunk count: {note}"
        );
        assert!(
            note.contains("persisted"),
            "note should report persistence: {note}"
        );
    }

    #[test]
    fn build_note_too_large_carries_remedy() {
        let note = bm25_build_note(
            1000,
            &Ok(crate::core::bm25_index::SaveOutcome::SkippedTooLarge {
                compressed_bytes: 600 * 1024 * 1024,
                limit_bytes: 512 * 1024 * 1024,
            }),
        );
        assert!(
            note.contains("NOT persisted"),
            "must flag non-persistence: {note}"
        );
        assert!(
            note.contains("LEAN_CTX_BM25_MAX_CACHE_MB") && note.contains("reindex"),
            "too-large note must carry an actionable remedy: {note}"
        );
    }

    #[test]
    fn build_note_persist_error_is_reported() {
        let note = bm25_build_note(7, &Err(std::io::Error::other("disk full")));
        assert!(note.contains("persisting failed"), "note: {note}");
        assert!(
            note.contains("disk full"),
            "note should include the io error: {note}"
        );
    }

    #[test]
    fn bm25_summary_unknown_project_is_idle() {
        let tmp = tempfile::tempdir().unwrap();
        let summary = bm25_summary(tmp.path().to_string_lossy().as_ref());
        assert_eq!(summary.state, "idle");
        assert!(summary.note.is_none());
        assert!(summary.last_error.is_none());
    }
}
