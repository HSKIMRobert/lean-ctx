use serde::Deserialize;

use super::helpers::{
    detect_project_root_for_dashboard, json_err, json_ok, normalize_dashboard_demo_path,
};

pub(super) fn handle(
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
            let pressure = ledger.pressure();
            let effective_used =
                (pressure.utilization * ledger.window_size as f64).round() as usize;
            let budget = crate::core::context_field::TokenBudget {
                total: ledger.window_size,
                used: effective_used,
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
                "budget_total": ledger.window_size,
                "budget_used": effective_used,
                "budget_remaining": pressure.remaining_tokens,
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
        "/api/context-bounce" => {
            let payload = if let Ok(bt) = crate::core::bounce_tracker::global().lock() {
                serde_json::json!({
                    "summary": bt.format_summary(),
                    "total_bounces": bt.total_bounces(),
                    "total_wasted_tokens": bt.total_wasted_tokens(),
                })
            } else {
                serde_json::json!({ "error": "lock failed" })
            };
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/context-client" => {
            let caps = crate::core::client_capabilities::current();
            let payload = serde_json::json!({
                "client_id": caps.client_id,
                "tier": caps.tier(),
                "resources": caps.resources,
                "prompts": caps.prompts,
                "elicitation": caps.elicitation,
                "sampling": caps.sampling,
                "dynamic_tools": caps.dynamic_tools,
                "max_tools": caps.max_tools,
                "summary": caps.format_summary(),
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/context-pressure" => {
            let ledger = crate::core::context_ledger::ContextLedger::load();
            let pressure = ledger.pressure();
            let adjusted_saved = ledger.adjusted_total_saved();
            let eviction_candidates = ledger.eviction_candidates_by_phi(5);
            let payload = serde_json::json!({
                "utilization": pressure.utilization,
                "remaining_tokens": pressure.remaining_tokens,
                "recommendation": format!("{:?}", pressure.recommendation),
                "total_sent": ledger.total_tokens_sent,
                "total_saved_raw": ledger.total_tokens_saved,
                "total_saved_adjusted": adjusted_saved,
                "window_size": ledger.window_size,
                "eviction_candidates": eviction_candidates,
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/context-dynamic-tools" => {
            let payload = if let Ok(state) = crate::server::dynamic_tools::global().lock() {
                serde_json::json!({
                    "active_categories": state.active_categories(),
                    "all_categories": crate::server::dynamic_tools::DynamicToolState::all_categories(),
                    "supports_list_changed": state.supports_list_changed(),
                })
            } else {
                serde_json::json!({ "error": "lock failed" })
            };
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Some(("200 OK", "application/json", json))
        }
        "/api/context-radar" => {
            let data_dir = crate::core::data_dir::lean_ctx_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let window = crate::core::context_radar::default_window_for_client("cursor");
            let radar = crate::core::context_radar::ContextRadar::load(&data_dir, window);
            let breakdown = radar.budget_breakdown();
            let recent_events: Vec<&crate::core::context_radar::RadarEvent> =
                radar.events.iter().rev().take(100).collect();
            let rules_files: Vec<serde_json::Value> = radar
                .rules_tokens
                .files
                .iter()
                .map(|(path, tokens)| serde_json::json!({ "path": path, "tokens": tokens }))
                .collect();
            let payload = serde_json::json!({
                "breakdown": breakdown,
                "rules": {
                    "files": rules_files,
                    "total_tokens": radar.rules_tokens.total,
                },
                "events_total": radar.events.len(),
                "recent_events": recent_events,
            });
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
    let project_root = detect_project_root_for_dashboard();
    let root_path = std::path::PathBuf::from(&project_root);

    let mut ledger = crate::core::context_ledger::ContextLedger::load();
    let mut overlays = crate::core::context_overlay::OverlayStore::load_project(&root_path);

    let action = match req.action.as_str() {
        "priority" => "set_priority".to_string(),
        other => other.to_string(),
    };

    if action == "expire" {
        let root_path = std::path::PathBuf::from(&project_root);
        let target = crate::core::context_field::ContextItemId::from_file(&path_norm);
        let secs: u64 = req
            .value
            .as_ref()
            .and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0);
        let op = crate::core::context_overlay::OverlayOp::Expire { after_secs: secs };
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
        return ("200 OK", "application/json", json_ok());
    }

    let mut args = serde_json::Map::new();
    args.insert("action".into(), serde_json::Value::String(action));
    args.insert(
        "target".into(),
        serde_json::Value::String(path_norm.clone()),
    );
    args.insert("scope".into(), serde_json::Value::String("project".into()));
    if let Some(v) = &req.value {
        let val_str = match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => {
                if *b {
                    "verbatim".to_string()
                } else {
                    "false".to_string()
                }
            }
            other => other.to_string(),
        };
        args.insert("value".into(), serde_json::Value::String(val_str));
    }

    let _result = crate::tools::ctx_control::handle(Some(&args), &mut ledger, &mut overlays);
    ledger.save();
    if let Err(e) = overlays.save_project(&root_path) {
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
