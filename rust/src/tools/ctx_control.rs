//! ctx_control -- Universal context manipulation tool.
//!
//! Single entry point for include/exclude/pin/rewrite/set_view operations.
//! Delegates to the Overlay and Ledger systems.

use serde_json::Value;

use crate::core::context_field::{ContextItemId, ContextState, ViewKind};
use crate::core::context_ledger::ContextLedger;
use crate::core::context_overlay::{
    ContextOverlay, OverlayAuthor, OverlayId, OverlayOp, OverlayScope, OverlayStore,
};

pub fn handle(
    args: Option<&serde_json::Map<String, Value>>,
    ledger: &mut ContextLedger,
    overlays: &mut OverlayStore,
) -> String {
    let action = get_str(args, "action").unwrap_or_default();
    let target = get_str(args, "target").unwrap_or_default();
    let value = get_str(args, "value");
    let scope_str = get_str(args, "scope").unwrap_or_else(|| "session".to_string());
    let reason = get_str(args, "reason").unwrap_or_else(|| action.clone());

    let scope = match scope_str.as_str() {
        "call" => OverlayScope::Call,
        "project" => OverlayScope::Project,
        "global" => OverlayScope::Global,
        _ => OverlayScope::Session,
    };

    let item_id = resolve_target(&target, ledger);

    match action.as_str() {
        "exclude" => {
            let op = OverlayOp::Exclude { reason: reason.clone() };
            apply_overlay(overlays, &item_id, op, scope);
            ledger.set_state(&target, ContextState::Excluded);
            format!("[ctx_control] excluded {target}: {reason}")
        }
        "include" => {
            let op = OverlayOp::Include;
            apply_overlay(overlays, &item_id, op, scope);
            ledger.set_state(&target, ContextState::Included);
            format!("[ctx_control] included {target}")
        }
        "pin" => {
            let verbatim = value.as_deref() == Some("verbatim");
            let op = OverlayOp::Pin { verbatim };
            apply_overlay(overlays, &item_id, op, scope);
            ledger.set_state(&target, ContextState::Pinned);
            format!("[ctx_control] pinned {target}")
        }
        "unpin" => {
            let op = OverlayOp::Unpin;
            apply_overlay(overlays, &item_id, op, scope);
            ledger.set_state(&target, ContextState::Included);
            format!("[ctx_control] unpinned {target}")
        }
        "set_view" => {
            let view_str = value.unwrap_or_else(|| "full".to_string());
            let view = ViewKind::parse(&view_str);
            let op = OverlayOp::SetView(view);
            apply_overlay(overlays, &item_id, op, scope);
            format!("[ctx_control] set view for {target} to {view_str}")
        }
        "set_priority" => {
            let priority: f64 = value
                .as_deref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.5);
            let op = OverlayOp::SetPriority(priority);
            apply_overlay(overlays, &item_id, op, scope);
            ledger.update_phi(&target, priority);
            format!("[ctx_control] set priority for {target} to {priority:.2}")
        }
        "mark_outdated" => {
            let op = OverlayOp::MarkOutdated;
            apply_overlay(overlays, &item_id, op, scope);
            ledger.set_state(&target, ContextState::Stale);
            format!("[ctx_control] marked {target} as outdated")
        }
        "reset" => {
            overlays.remove_for_item(&item_id);
            ledger.set_state(&target, ContextState::Included);
            format!("[ctx_control] reset all overlays for {target}")
        }
        "list" => {
            let items = overlays.all();
            if items.is_empty() {
                "[ctx_control] no active overlays".to_string()
            } else {
                let mut out = format!("[ctx_control] {} active overlays:\n", items.len());
                for ov in items {
                    let stale_tag = if ov.stale { " [STALE]" } else { "" };
                    out.push_str(&format!(
                        "  {} -> {:?} ({:?}){}\n",
                        ov.target, ov.operation, ov.scope, stale_tag
                    ));
                }
                out
            }
        }
        "history" => {
            let history = overlays.for_item(&item_id);
            if history.is_empty() {
                format!("[ctx_control] no overlay history for {target}")
            } else {
                let mut out = format!(
                    "[ctx_control] {} overlays for {target}:\n",
                    history.len()
                );
                for ov in history {
                    out.push_str(&format!(
                        "  {} {:?} at {} ({:?})\n",
                        ov.id, ov.operation, ov.created_at, ov.scope
                    ));
                }
                out
            }
        }
        _ => format!("[ctx_control] unknown action: {action}. valid: exclude|include|pin|unpin|set_view|set_priority|mark_outdated|reset|list|history"),
    }
}

fn resolve_target(target: &str, ledger: &ContextLedger) -> ContextItemId {
    if target.starts_with("file:")
        || target.starts_with("shell:")
        || target.starts_with("knowledge:")
    {
        ContextItemId(target.to_string())
    } else if let Some(stripped) = target.strip_prefix('@') {
        ContextItemId::from_file(stripped)
    } else if let Some(entry) = ledger.entries.iter().find(|e| e.path == target) {
        entry
            .id
            .clone()
            .unwrap_or_else(|| ContextItemId::from_file(target))
    } else {
        ContextItemId::from_file(target)
    }
}

fn apply_overlay(
    overlays: &mut OverlayStore,
    item_id: &ContextItemId,
    operation: OverlayOp,
    scope: OverlayScope,
) {
    let overlay = ContextOverlay {
        id: OverlayId::generate(item_id),
        target: item_id.clone(),
        operation,
        scope,
        before_hash: String::new(),
        author: OverlayAuthor::Agent("mcp".to_string()),
        created_at: chrono::Utc::now(),
        stale: false,
    };
    overlays.add(overlay);
}

fn get_str(args: Option<&serde_json::Map<String, Value>>, key: &str) -> Option<String> {
    args?
        .get(key)?
        .as_str()
        .map(std::string::ToString::to_string)
}
