use serde::Deserialize;

use super::helpers::{
    detect_project_root_for_dashboard, json_err, json_ok, normalize_dashboard_demo_path,
};

pub fn handle(
    path: &str,
    query_str: &str,
    method: &str,
    body: &str,
) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/context-overlay" if method.eq_ignore_ascii_case("POST") => {
            Some(post_context_overlay(body))
        }
        "/api/context-policy" if method.eq_ignore_ascii_case("POST") => {
            Some(post_context_policy(body))
        }
        _ => get_routes(path, query_str),
    }
}

fn get_routes(path: &str, _query_str: &str) -> Option<(&'static str, &'static str, String)> {
    match path {
        "/api/context-ledger" => {
            let ledger = crate::core::context_ledger::ContextLedger::load();
            let pressure = ledger.pressure();
            let payload = serde_json::json!({
                "window_size": ledger.window_size,
                "entries_count": ledger.entries.len(),
                "total_tokens_sent": ledger.total_tokens_sent,
                "total_tokens_saved": ledger.total_tokens_saved,
                "compression_ratio": ledger.compression_ratio(),
                "pressure": {
                    "utilization": pressure.utilization,
                    "remaining_tokens": pressure.remaining_tokens,
                    "recommendation": format!("{:?}", pressure.recommendation),
                },
                "mode_distribution": ledger.mode_distribution(),
                "entries": ledger.entries.iter().take(50).collect::<Vec<_>>(),
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/context-control" => {
            let project_root = detect_project_root_for_dashboard();
            let mut ledger = crate::core::context_ledger::ContextLedger::load();
            let mut overlays = crate::core::context_overlay::OverlayStore::load_project(
                &std::path::PathBuf::from(&project_root),
            );
            let mut args = serde_json::Map::new();
            args.insert(
                "action".to_string(),
                serde_json::Value::String("list".to_string()),
            );
            let result = crate::tools::ctx_control::handle(Some(&args), &mut ledger, &mut overlays);
            ledger.save();
            let _ = overlays.save_project(&std::path::PathBuf::from(&project_root));
            let payload = serde_json::json!({
                "result": result,
                "overlays": overlays.all(),
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/context-field" => {
            let ledger = crate::core::context_ledger::ContextLedger::load();
            let field = crate::core::context_field::ContextField::new();
            let budget = crate::core::context_field::TokenBudget {
                total: ledger.window_size,
                used: ledger.total_tokens_sent,
            };
            let items: Vec<serde_json::Value> = ledger
                .entries
                .iter()
                .map(|e| {
                    let phi = e.phi.unwrap_or_else(|| {
                        field.compute_phi(&crate::core::context_field::FieldSignals {
                            relevance: 0.3,
                            ..Default::default()
                        })
                    });
                    serde_json::json!({
                        "path": e.path,
                        "phi": phi,
                        "state": e.state,
                        "view": e.active_view,
                        "tokens": e.sent_tokens,
                        "kind": e.kind,
                    })
                })
                .collect();
            let payload = serde_json::json!({
                "temperature": budget.temperature(),
                "budget_total": budget.total,
                "budget_used": budget.used,
                "budget_remaining": budget.remaining(),
                "items": items,
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/context-handles" => {
            let ledger = crate::core::context_ledger::ContextLedger::load();
            let project_root = detect_project_root_for_dashboard();
            let policies = crate::core::context_policies::PolicySet::load_project(
                &std::path::PathBuf::from(&project_root),
            );
            let candidates = crate::tools::ctx_plan::plan_to_candidates(&ledger, &policies);
            let mut registry = crate::core::context_handles::HandleRegistry::new();
            for c in &candidates {
                if c.state == crate::core::context_field::ContextState::Excluded {
                    continue;
                }
                let summary = format!("{} {}", c.path, c.selected_view.as_str());
                registry.register(
                    c.id.clone(),
                    c.kind,
                    &c.path,
                    &summary,
                    &c.view_costs,
                    c.phi,
                    c.pinned,
                );
            }
            let json = serde_json::to_string(&registry).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/context-overlay-history" => {
            let project_root = detect_project_root_for_dashboard();
            let store = crate::core::context_overlay::OverlayStore::load_project(
                &std::path::PathBuf::from(&project_root),
            );
            let json = serde_json::to_string(store.all()).unwrap_or_else(|_| "[]".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/context-plan" => {
            let ledger = crate::core::context_ledger::ContextLedger::load();
            let project_root = detect_project_root_for_dashboard();
            let policies = crate::core::context_policies::PolicySet::load_project(
                &std::path::PathBuf::from(&project_root),
            );
            let text = crate::tools::ctx_plan::handle(None, &ledger, &policies);
            let payload = serde_json::json!({ "plan": text });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        _ => None,
    }
}

#[derive(Deserialize)]
struct OverlayReq {
    action: String,
    path: String,
    #[serde(default)]
    value: Option<serde_json::Value>,
}

fn post_context_overlay(body: &str) -> (&'static str, &'static str, String) {
    let req: OverlayReq = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return (
                "400 Bad Request",
                "application/json",
                json_err(&format!("invalid JSON: {e}")),
            );
        }
    };
    let path_norm = normalize_dashboard_demo_path(req.path.trim());
    if path_norm.is_empty() {
        return (
            "400 Bad Request",
            "application/json",
            json_err("path is required"),
        );
    }
    let target = crate::core::context_field::ContextItemId::from_file(&path_norm);
    let op = match req.action.as_str() {
        "pin" => {
            let verbatim = req
                .value
                .as_ref()
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            crate::core::context_overlay::OverlayOp::Pin { verbatim }
        }
        "exclude" => crate::core::context_overlay::OverlayOp::Exclude {
            reason: req
                .value
                .as_ref()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "dashboard".to_string()),
        },
        "include" => crate::core::context_overlay::OverlayOp::Include,
        "unpin" => crate::core::context_overlay::OverlayOp::Unpin,
        "set_view" => {
            let mode = req
                .value
                .as_ref()
                .and_then(|v| v.as_str())
                .unwrap_or("full");
            crate::core::context_overlay::OverlayOp::SetView(
                crate::core::context_field::ViewKind::parse(mode),
            )
        }
        "priority" => {
            let v = match &req.value {
                Some(serde_json::Value::Number(n)) => n.as_f64(),
                Some(serde_json::Value::String(s)) => s.parse().ok(),
                _ => None,
            };
            let Some(p) = v else {
                return (
                    "400 Bad Request",
                    "application/json",
                    json_err("priority requires numeric value"),
                );
            };
            crate::core::context_overlay::OverlayOp::SetPriority(p)
        }
        "mark_outdated" => crate::core::context_overlay::OverlayOp::MarkOutdated,
        "expire" => {
            let secs: Option<u64> = match &req.value {
                Some(serde_json::Value::Number(n)) => n.as_u64(),
                Some(serde_json::Value::String(s)) => s.parse().ok(),
                _ => None,
            };
            let Some(after_secs) = secs else {
                return (
                    "400 Bad Request",
                    "application/json",
                    json_err("expire requires numeric seconds in value"),
                );
            };
            crate::core::context_overlay::OverlayOp::Expire { after_secs }
        }
        _ => {
            return (
                "400 Bad Request",
                "application/json",
                json_err("unknown action"),
            );
        }
    };

    let project_root = detect_project_root_for_dashboard();
    let root_path = std::path::PathBuf::from(&project_root);
    let mut store = crate::core::context_overlay::OverlayStore::load_project(&root_path);
    store.add(crate::core::context_overlay::ContextOverlay::new(
        target,
        op,
        crate::core::context_overlay::OverlayScope::Project,
        String::new(),
        crate::core::context_overlay::OverlayAuthor::User,
    ));
    if let Err(e) = store.save_project(&root_path) {
        return (
            "500 Internal Server Error",
            "application/json",
            json_err(&e),
        );
    }
    ("200 OK", "application/json", json_ok())
}

#[derive(Deserialize)]
struct PolicyReq {
    action: String,
    rule: serde_json::Value,
}

fn post_context_policy(body: &str) -> (&'static str, &'static str, String) {
    let req: PolicyReq = match serde_json::from_str(body) {
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
    let root_path = std::path::PathBuf::from(&project_root);
    let mut policies = crate::core::context_policies::PolicySet::load_project(&root_path);

    match req.action.as_str() {
        "add" => {
            let rule: crate::core::context_policies::ContextPolicy =
                match serde_json::from_value(req.rule) {
                    Ok(p) => p,
                    Err(e) => {
                        return (
                            "400 Bad Request",
                            "application/json",
                            json_err(&format!("invalid rule: {e}")),
                        );
                    }
                };
            if rule.name.trim().is_empty() || rule.match_pattern.trim().is_empty() {
                return (
                    "400 Bad Request",
                    "application/json",
                    json_err("rule.name and rule.match_pattern are required"),
                );
            }
            policies.policies.push(rule);
            if let Err(e) = policies.save_project(&root_path) {
                return (
                    "500 Internal Server Error",
                    "application/json",
                    json_err(&e),
                );
            }
            ("200 OK", "application/json", json_ok())
        }
        "remove" => {
            let name = req
                .rule
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let Some(name) = name else {
                return (
                    "400 Bad Request",
                    "application/json",
                    json_err("remove requires rule.name"),
                );
            };
            let before = policies.policies.len();
            policies.policies.retain(|p| p.name != name);
            if policies.policies.len() == before {
                return (
                    "400 Bad Request",
                    "application/json",
                    json_err("no policy matched name"),
                );
            }
            if let Err(e) = policies.save_project(&root_path) {
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
