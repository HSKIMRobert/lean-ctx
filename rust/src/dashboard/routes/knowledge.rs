use serde::Deserialize;

use super::helpers::{detect_project_root_for_dashboard, json_err, json_ok};

pub fn handle(
    path: &str,
    query_str: &str,
    method: &str,
    body: &str,
) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/knowledge/edit" if method.eq_ignore_ascii_case("POST") => {
            Some(post_knowledge_edit(body))
        }
        "/api/knowledge-relations/edit" if method.eq_ignore_ascii_case("POST") => {
            Some(post_knowledge_relations_edit(body))
        }
        _ => get_routes(path, query_str),
    }
}

fn get_routes(path: &str, _query_str: &str) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/knowledge" => {
            let project_root = detect_project_root_for_dashboard();
            let policy = crate::core::config::Config::load()
                .memory_policy_effective()
                .unwrap_or_default();
            let _ = crate::core::knowledge::ProjectKnowledge::migrate_legacy_empty_root(
                &project_root,
                &policy,
            );

            let mut knowledge =
                crate::core::knowledge::ProjectKnowledge::load_or_create(&project_root);
            if knowledge.facts.is_empty() {
                // Keep /api/knowledge fast: avoid forcing a full index build here.
                let idx = crate::core::graph_index::ProjectIndex::load(&project_root);
                if crate::core::knowledge_bootstrap::bootstrap_if_empty(
                    &mut knowledge,
                    &project_root,
                    idx.as_ref(),
                    &policy,
                ) {
                    let _ = knowledge.save();
                }
            }
            let json = serde_json::to_string(&knowledge).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/knowledge-relations" => {
            let project_root = detect_project_root_for_dashboard();
            let policy = crate::core::config::Config::load()
                .memory_policy_effective()
                .unwrap_or_default();

            let knowledge = crate::core::knowledge::ProjectKnowledge::load_or_create(&project_root);
            let graph = crate::core::knowledge_relations::KnowledgeRelationGraph::load_or_create(
                &knowledge.project_hash,
            );

            let current_ids: std::collections::HashSet<String> = knowledge
                .facts
                .iter()
                .filter(|f| f.is_current())
                .map(|f| format!("{}/{}", f.category, f.key))
                .collect();

            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut edges: Vec<serde_json::Value> = Vec::new();

            let mut push_edge = |from: String, to: String, kind: String, derived: bool| {
                if from.trim().is_empty() || to.trim().is_empty() || from == to {
                    return;
                }
                if !current_ids.contains(&from) || !current_ids.contains(&to) {
                    return;
                }
                let key = format!("{from}|{kind}|{to}");
                if !seen.insert(key) {
                    return;
                }
                edges.push(serde_json::json!({
                    "from": from,
                    "to": to,
                    "kind": kind,
                    "derived": derived,
                }));
            };

            // Explicit user-managed relations.
            for e in &graph.edges {
                push_edge(e.from.id(), e.to.id(), e.kind.as_str().to_string(), false);
            }

            // Derived: `supersedes` links (stored on facts).
            for f in knowledge.facts.iter().filter(|f| f.is_current()) {
                let Some(to) = f
                    .supersedes
                    .as_deref()
                    .and_then(crate::core::knowledge_relations::parse_node_ref)
                else {
                    continue;
                };
                let from = format!("{}/{}", f.category, f.key);
                push_edge(from, to.id(), "supersedes".to_string(), true);
            }

            // Derived: soft references in values like `category/key` or `category:key`.
            for f in knowledge.facts.iter().filter(|f| f.is_current()) {
                let from = format!("{}/{}", f.category, f.key);
                for raw in f.value.split_whitespace() {
                    let tok = raw.trim_matches(|c: char| {
                        !c.is_ascii_alphanumeric() && c != '/' && c != ':' && c != '_' && c != '-'
                    });
                    let Some(to) = crate::core::knowledge_relations::parse_node_ref(tok) else {
                        continue;
                    };
                    if to.id() == from {
                        continue;
                    }
                    push_edge(from.clone(), to.id(), "related_to".to_string(), true);
                }
            }

            let max_edges = policy.knowledge.max_facts.saturating_mul(8);
            if max_edges > 0 && edges.len() > max_edges {
                edges.truncate(max_edges);
            }

            let payload = serde_json::json!({
                "project_root": project_root,
                "project_hash": knowledge.project_hash,
                "edges": edges,
                "explicit_edges_total": graph.edges.len(),
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/gotchas" => {
            let project_root = detect_project_root_for_dashboard();
            let store = crate::core::gotcha_tracker::GotchaStore::load(&project_root);
            let json = serde_json::to_string(&store).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        _ => None,
    }
}

#[derive(Deserialize)]
struct KnowledgeEditReq {
    action: String,
    fact_index: usize,
    #[serde(default)]
    value: Option<String>,
}

fn post_knowledge_edit(body: &str) -> (&'static str, &'static str, String) {
    let req: KnowledgeEditReq = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return (
                "400 Bad Request",
                "application/json",
                json_err(&format!("invalid JSON: {e}")),
            );
        }
    };
    let project_root = detect_project_root_for_dashboard();
    let policy = crate::core::config::Config::load()
        .memory_policy_effective()
        .unwrap_or_default();
    let _ =
        crate::core::knowledge::ProjectKnowledge::migrate_legacy_empty_root(&project_root, &policy);
    let mut knowledge = crate::core::knowledge::ProjectKnowledge::load_or_create(&project_root);

    let current_idxs: Vec<usize> = knowledge
        .facts
        .iter()
        .enumerate()
        .filter(|(_, f)| f.is_current())
        .map(|(i, _)| i)
        .collect();
    let Some(&real_idx) = current_idxs.get(req.fact_index) else {
        return (
            "400 Bad Request",
            "application/json",
            json_err("fact_index out of range"),
        );
    };

    match req.action.as_str() {
        "archive" => {
            use chrono::Utc;
            knowledge.facts[real_idx].valid_until = Some(Utc::now());
            knowledge.facts[real_idx].valid_from = knowledge.facts[real_idx]
                .valid_from
                .or(Some(knowledge.facts[real_idx].created_at));
        }
        "delete" => {
            knowledge.facts.swap_remove(real_idx);
        }
        "update_confidence" => {
            let Some(ref raw) = req.value else {
                return (
                    "400 Bad Request",
                    "application/json",
                    json_err("value required for update_confidence"),
                );
            };
            let Ok(c) = raw.parse::<f32>() else {
                return (
                    "400 Bad Request",
                    "application/json",
                    json_err("value must be a float"),
                );
            };
            knowledge.facts[real_idx].confidence = c.clamp(0.0, 1.0);
        }
        _ => {
            return (
                "400 Bad Request",
                "application/json",
                json_err("unknown action"),
            );
        }
    }

    knowledge.updated_at = chrono::Utc::now();
    if let Err(e) = knowledge.save() {
        return (
            "500 Internal Server Error",
            "application/json",
            json_err(&e),
        );
    }
    ("200 OK", "application/json", json_ok())
}

#[derive(Deserialize)]
struct RelationEditReq {
    action: String,
    relation: RelationBody,
}

#[derive(Deserialize)]
struct RelationBody {
    from: String,
    to: String,
    kind: String,
}

fn post_knowledge_relations_edit(body: &str) -> (&'static str, &'static str, String) {
    let req: RelationEditReq = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return (
                "400 Bad Request",
                "application/json",
                json_err(&format!("invalid JSON: {e}")),
            );
        }
    };
    let Some(from) = crate::core::knowledge_relations::parse_node_ref(&req.relation.from) else {
        return (
            "400 Bad Request",
            "application/json",
            json_err("invalid relation.from"),
        );
    };
    let Some(to) = crate::core::knowledge_relations::parse_node_ref(&req.relation.to) else {
        return (
            "400 Bad Request",
            "application/json",
            json_err("invalid relation.to"),
        );
    };
    let Some(kind) = crate::core::knowledge_relations::KnowledgeEdgeKind::parse(&req.relation.kind)
    else {
        return (
            "400 Bad Request",
            "application/json",
            json_err("invalid relation.kind"),
        );
    };

    let project_root = detect_project_root_for_dashboard();
    let knowledge = crate::core::knowledge::ProjectKnowledge::load_or_create(&project_root);
    let mut graph = crate::core::knowledge_relations::KnowledgeRelationGraph::load_or_create(
        &knowledge.project_hash,
    );

    match req.action.as_str() {
        "add" => {
            let _ = graph.upsert_edge(from, to, kind, "dashboard");
            let max_edges = crate::core::config::Config::load()
                .memory_policy_effective()
                .unwrap_or_default()
                .knowledge
                .max_facts
                .saturating_mul(8);
            let _ = graph.enforce_cap(max_edges);
            if let Err(e) = graph.save() {
                return (
                    "500 Internal Server Error",
                    "application/json",
                    json_err(&e),
                );
            }
            ("200 OK", "application/json", json_ok())
        }
        "delete" => {
            let n = graph.remove_edge(&from, &to, Some(kind));
            if n == 0 {
                return (
                    "400 Bad Request",
                    "application/json",
                    json_err("no matching relation"),
                );
            }
            if let Err(e) = graph.save() {
                return (
                    "500 Internal Server Error",
                    "application/json",
                    json_err(&e),
                );
            }
            ("200 OK", "application/json", json_ok())
        }
        _ => (
            "400 Bad Request",
            "application/json",
            json_err("unknown action"),
        ),
    }
}
