use chrono::Utc;
use serde::Deserialize;

use super::helpers::{detect_project_root_for_dashboard, extract_query_param, json_err, json_ok};

pub fn handle(
    path: &str,
    query_str: &str,
    method: &str,
    body: &str,
) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/session/note" if method.eq_ignore_ascii_case("POST") => Some(post_session_note(body)),
        "/api/episodes/annotate" if method.eq_ignore_ascii_case("POST") => {
            Some(post_episodes_annotate(body))
        }
        _ => get_routes(path, query_str),
    }
}

fn get_routes(path: &str, query_str: &str) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/episodes" => {
            let root = detect_project_root_for_dashboard();
            let hash = crate::core::project_hash::hash_project_root(&root);
            let store = crate::core::episodic_memory::EpisodicStore::load_or_create(&hash);
            let stats = store.stats();
            let recent: Vec<_> = store.recent(20).into_iter().cloned().collect();
            let payload = serde_json::json!({
                "project_root": root,
                "project_hash": hash,
                "stats": {
                    "total_episodes": stats.total_episodes,
                    "successes": stats.successes,
                    "failures": stats.failures,
                    "success_rate": stats.success_rate,
                    "total_tokens": stats.total_tokens,
                },
                "recent": recent,
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/procedures" => {
            let root = detect_project_root_for_dashboard();
            let hash = crate::core::project_hash::hash_project_root(&root);
            let store = crate::core::procedural_memory::ProceduralStore::load_or_create(&hash);
            let task = extract_query_param(query_str, "task").or_else(|| {
                crate::core::session::SessionState::load_latest_for_project_root(&root)
                    .and_then(|s| s.task.map(|t| t.description))
            });
            let suggestions: Vec<serde_json::Value> = task.as_deref().map_or(Vec::new(), |t| {
                store
                    .suggest(t)
                    .into_iter()
                    .take(10)
                    .map(|p| {
                        serde_json::json!({
                            "id": p.id,
                            "name": p.name,
                            "description": p.description,
                            "confidence": p.confidence,
                            "times_used": p.times_used,
                            "times_succeeded": p.times_succeeded,
                            "success_rate": p.success_rate(),
                            "steps": p.steps,
                            "activation_keywords": p.activation_keywords,
                            "last_used": p.last_used,
                            "created_at": p.created_at,
                        })
                    })
                    .collect()
            });
            let payload = serde_json::json!({
                "project_root": root,
                "project_hash": hash,
                "total_procedures": store.procedures.len(),
                "task": task,
                "suggestions": suggestions,
                "procedures": store.procedures,
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/session" => {
            let session = crate::core::session::SessionState::load_latest().unwrap_or_default();
            let json = serde_json::to_string(&session)
                .unwrap_or_else(|_| "{\"error\":\"failed to serialize session\"}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/intent" => {
            let session_path = crate::core::data_dir::lean_ctx_data_dir()
                .ok()
                .map(|d| d.join("sessions"));
            let mut intent_data = serde_json::json!({"active": false});
            if let Some(dir) = session_path {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    let mut newest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
                    for e in entries.flatten() {
                        if e.path().extension().is_some_and(|ext| ext == "json") {
                            if let Ok(meta) = e.metadata() {
                                let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                                if newest.as_ref().is_none_or(|(t, _)| mtime > *t) {
                                    newest = Some((mtime, e.path()));
                                }
                            }
                        }
                    }
                    if let Some((_, path)) = newest {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(session) = serde_json::from_str::<serde_json::Value>(&content)
                            {
                                if let Some(intent) = session.get("active_structured_intent") {
                                    if !intent.is_null() {
                                        intent_data = serde_json::json!({
                                            "active": true,
                                            "intent": intent,
                                            "session_file": path.file_name().unwrap_or_default().to_string_lossy(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let json = serde_json::to_string(&intent_data).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        _ => None,
    }
}

#[derive(Deserialize)]
struct SessionNoteReq {
    note: String,
}

fn post_session_note(body: &str) -> (&'static str, &'static str, String) {
    let req: SessionNoteReq = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return (
                "400 Bad Request",
                "application/json",
                json_err(&format!("invalid JSON: {e}")),
            );
        }
    };
    let note = req.note.trim();
    if note.is_empty() {
        return (
            "400 Bad Request",
            "application/json",
            json_err("note must not be empty"),
        );
    }
    let Some(mut session) = crate::core::session::SessionState::load_latest() else {
        return (
            "400 Bad Request",
            "application/json",
            json_err("no session to attach note to"),
        );
    };
    let now = Utc::now();
    session.updated_at = now;
    session.progress.push(crate::core::session::ProgressEntry {
        action: "note".to_string(),
        detail: Some(note.to_string()),
        timestamp: now,
    });
    if let Err(e) = session.save() {
        return (
            "500 Internal Server Error",
            "application/json",
            json_err(&e),
        );
    }
    ("200 OK", "application/json", json_ok())
}

#[derive(Deserialize)]
struct EpisodeAnnotateReq {
    episode_index: usize,
    outcome: String,
}

fn post_episodes_annotate(body: &str) -> (&'static str, &'static str, String) {
    let req: EpisodeAnnotateReq = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return (
                "400 Bad Request",
                "application/json",
                json_err(&format!("invalid JSON: {e}")),
            );
        }
    };
    let root = detect_project_root_for_dashboard();
    let hash = crate::core::project_hash::hash_project_root(&root);
    let mut store = crate::core::episodic_memory::EpisodicStore::load_or_create(&hash);
    let n = store.episodes.len();
    if n == 0 || req.episode_index >= n {
        return (
            "400 Bad Request",
            "application/json",
            json_err("episode_index out of range (newest-first: 0 = most recent)"),
        );
    }
    // Align with /api/episodes `recent` ordering (most recent first).
    let real_idx = n - 1 - req.episode_index;
    let new_outcome = match req.outcome.to_lowercase().as_str() {
        "success" => crate::core::episodic_memory::Outcome::Success {
            tests_passed: false,
        },
        "failure" => crate::core::episodic_memory::Outcome::Failure {
            error: "annotated failure".to_string(),
        },
        "neutral" => crate::core::episodic_memory::Outcome::Unknown,
        _ => {
            return (
                "400 Bad Request",
                "application/json",
                json_err("outcome must be success, failure, or neutral"),
            );
        }
    };
    store.episodes[real_idx].outcome = new_outcome;
    if let Err(e) = store.save() {
        return (
            "500 Internal Server Error",
            "application/json",
            json_err(&e),
        );
    }
    ("200 OK", "application/json", json_ok())
}
