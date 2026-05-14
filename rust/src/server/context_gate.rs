use crate::core::context_field::{ContextItemId, ContextState};
use crate::core::context_overlay::{OverlayOp, OverlayStore};

#[derive(Debug, Clone)]
pub struct PreDispatchResult {
    pub overridden_mode: Option<String>,
    pub reason: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct PostDispatchResult {
    pub eviction_hint: Option<String>,
    pub elicitation_hint: Option<String>,
}

pub fn pre_dispatch_read(
    path: &str,
    requested_mode: &str,
    task: Option<&str>,
    project_root: Option<&str>,
) -> PreDispatchResult {
    if requested_mode == "diff" {
        return PreDispatchResult {
            overridden_mode: None,
            reason: None,
        };
    }

    if let Some(root) = project_root {
        let overlay = OverlayStore::load_project(&std::path::PathBuf::from(root));
        if let Some(result) = check_overlay_mode_override(path, requested_mode, &overlay) {
            return result;
        }
    }

    if requested_mode == "full" {
        return PreDispatchResult {
            overridden_mode: None,
            reason: None,
        };
    }

    if let Ok(bt) = crate::core::bounce_tracker::global().lock() {
        if bt.should_force_full(path) {
            return PreDispatchResult {
                overridden_mode: Some("full".to_string()),
                reason: Some("bounce-prevention"),
            };
        }
    }

    if let Some(task_str) = task {
        let intent = crate::core::intent_engine::StructuredIntent::from_query(task_str);
        let norm = crate::core::pathutil::normalize_tool_path(path);
        let is_target = intent
            .targets
            .iter()
            .any(|t| norm.ends_with(t) || norm.contains(t));
        if is_target {
            return PreDispatchResult {
                overridden_mode: Some("full".to_string()),
                reason: Some("intent-target"),
            };
        }
    }

    if let Some(root) = project_root {
        if let Some(index) = try_load_graph(root) {
            let related = index.get_related(path, 1);
            if let Some(task_str) = task {
                let intent = crate::core::intent_engine::StructuredIntent::from_query(task_str);
                for target in &intent.targets {
                    let target_related = index.get_related(target, 1);
                    let norm = crate::core::pathutil::normalize_tool_path(path);
                    if target_related
                        .iter()
                        .any(|r| r.contains(&norm) || norm.contains(r))
                    {
                        return PreDispatchResult {
                            overridden_mode: Some("map".to_string()),
                            reason: Some("graph-direct-import"),
                        };
                    }
                }
            }
            if !related.is_empty() && requested_mode == "auto" {
                let reverse_deps = index.get_reverse_deps(path, 1);
                if reverse_deps.len() > 3 {
                    return PreDispatchResult {
                        overridden_mode: Some("map".to_string()),
                        reason: Some("graph-hub-file"),
                    };
                }
            }
        }
    }

    if let Some(root) = project_root {
        if let Some(knowledge) = crate::core::knowledge::ProjectKnowledge::load(root) {
            let norm = crate::core::pathutil::normalize_tool_path(path);
            let mentions = knowledge
                .facts
                .iter()
                .filter(|f| f.value.contains(&norm) || f.key.contains(&norm))
                .count();
            if mentions >= 3 {
                return PreDispatchResult {
                    overridden_mode: Some("map".to_string()),
                    reason: Some("knowledge-high-relevance"),
                };
            }
        }
    }

    PreDispatchResult {
        overridden_mode: None,
        reason: None,
    }
}

fn check_overlay_mode_override(
    path: &str,
    requested_mode: &str,
    overlay: &OverlayStore,
) -> Option<PreDispatchResult> {
    let item_id = ContextItemId::from_file(path);
    let overlays = overlay.for_item(&item_id);

    for ov in overlays.iter().rev() {
        match &ov.operation {
            OverlayOp::SetView(view) => {
                let mode_str = view.as_str();
                if mode_str != requested_mode {
                    return Some(PreDispatchResult {
                        overridden_mode: Some(mode_str.to_string()),
                        reason: Some("overlay-set-view"),
                    });
                }
            }
            OverlayOp::Pin { .. } if requested_mode != "full" => {
                return Some(PreDispatchResult {
                    overridden_mode: Some("full".to_string()),
                    reason: Some("pinned"),
                });
            }
            OverlayOp::Exclude { .. } if requested_mode != "signatures" => {
                return Some(PreDispatchResult {
                    overridden_mode: Some("signatures".to_string()),
                    reason: Some("excluded"),
                });
            }
            _ => {}
        }
    }
    None
}

pub fn post_dispatch_record(
    path: &str,
    mode: &str,
    original_tokens: usize,
    sent_tokens: usize,
    ledger: &mut crate::core::context_ledger::ContextLedger,
    overlay: &crate::core::context_overlay::OverlayStore,
) -> PostDispatchResult {
    ledger.record(path, mode, original_tokens, sent_tokens);

    let item_id = ContextItemId::from_file(path);
    let state = overlay.apply_to_state(&item_id, ContextState::Included);

    if state == ContextState::Excluded {
        return PostDispatchResult {
            eviction_hint: Some(format!("File '{path}' is excluded by overlay.")),
            elicitation_hint: None,
        };
    }

    let elicitation =
        super::elicitation::check_elicitation_needed(ledger, Some(path), Some(sent_tokens))
            .map(|s| s.format_fallback_hint());

    let pressure = ledger.pressure();
    if pressure.utilization > 0.9 {
        let candidates = ledger.eviction_candidates_by_phi(3);
        if !candidates.is_empty() {
            let names: Vec<_> = candidates
                .iter()
                .take(3)
                .map(|p| crate::core::protocol::shorten_path(p))
                .collect();
            return PostDispatchResult {
                eviction_hint: Some(format!(
                    "Context pressure {:.0}%. Consider evicting: {}",
                    pressure.utilization * 100.0,
                    names.join(", ")
                )),
                elicitation_hint: elicitation,
            };
        }
    }

    PostDispatchResult {
        eviction_hint: None,
        elicitation_hint: elicitation,
    }
}

fn try_load_graph(project_root: &str) -> Option<crate::core::graph_index::ProjectIndex> {
    crate::core::graph_index::ProjectIndex::load(project_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_dispatch_passthrough_for_full() {
        let result = pre_dispatch_read("src/main.rs", "full", None, None);
        assert!(result.overridden_mode.is_none());
    }

    #[test]
    fn pre_dispatch_passthrough_for_diff() {
        let result = pre_dispatch_read("src/main.rs", "diff", None, None);
        assert!(result.overridden_mode.is_none());
    }

    #[test]
    fn pre_dispatch_no_override_without_signals() {
        let result = pre_dispatch_read("src/unknown.rs", "auto", None, None);
        assert!(result.overridden_mode.is_none());
    }

    #[test]
    fn pre_dispatch_bounce_prevention_forces_full() {
        {
            let mut bt = crate::core::bounce_tracker::global().lock().unwrap();
            bt.set_seq(1);
            bt.record_read("src/bouncy.yml", "map", 30, 400);
            bt.set_seq(2);
            bt.record_read("src/bouncy.yml", "full", 400, 400);
            bt.set_seq(3);
            bt.record_read("a2.yml", "map", 30, 400);
            bt.set_seq(4);
            bt.record_read("a2.yml", "full", 400, 400);
            bt.set_seq(5);
            bt.record_read("a3.yml", "map", 30, 400);
            bt.set_seq(6);
            bt.record_read("a3.yml", "full", 400, 400);
        }
        let result = pre_dispatch_read("new.yml", "auto", None, None);
        assert_eq!(result.overridden_mode, Some("full".to_string()));
        assert_eq!(result.reason, Some("bounce-prevention"));
    }

    #[test]
    fn overlay_pin_forces_full_mode() {
        let dir = tempfile::tempdir().expect("tmp dir");
        let root = dir.path();
        let mut store = OverlayStore::new();
        let target = ContextItemId::from_file("src/important.rs");
        store.add(crate::core::context_overlay::ContextOverlay::new(
            target,
            OverlayOp::Pin { verbatim: false },
            crate::core::context_overlay::OverlayScope::Project,
            String::new(),
            crate::core::context_overlay::OverlayAuthor::User,
        ));
        store.save_project(root).unwrap();

        let result = pre_dispatch_read(
            "src/important.rs",
            "auto",
            None,
            Some(root.to_str().unwrap()),
        );
        assert_eq!(result.overridden_mode, Some("full".to_string()));
        assert_eq!(result.reason, Some("pinned"));
    }

    #[test]
    fn overlay_exclude_forces_signatures_mode() {
        let dir = tempfile::tempdir().expect("tmp dir");
        let root = dir.path();
        let mut store = OverlayStore::new();
        let target = ContextItemId::from_file("src/noisy.rs");
        store.add(crate::core::context_overlay::ContextOverlay::new(
            target,
            OverlayOp::Exclude {
                reason: "noise".to_string(),
            },
            crate::core::context_overlay::OverlayScope::Project,
            String::new(),
            crate::core::context_overlay::OverlayAuthor::User,
        ));
        store.save_project(root).unwrap();

        let result = pre_dispatch_read("src/noisy.rs", "auto", None, Some(root.to_str().unwrap()));
        assert_eq!(result.overridden_mode, Some("signatures".to_string()));
        assert_eq!(result.reason, Some("excluded"));
    }

    #[test]
    fn overlay_set_view_forces_specified_mode() {
        let dir = tempfile::tempdir().expect("tmp dir");
        let root = dir.path();
        let mut store = OverlayStore::new();
        let target = ContextItemId::from_file("src/big.rs");
        store.add(crate::core::context_overlay::ContextOverlay::new(
            target,
            OverlayOp::SetView(crate::core::context_field::ViewKind::Map),
            crate::core::context_overlay::OverlayScope::Project,
            String::new(),
            crate::core::context_overlay::OverlayAuthor::User,
        ));
        store.save_project(root).unwrap();

        let result = pre_dispatch_read("src/big.rs", "auto", None, Some(root.to_str().unwrap()));
        assert_eq!(result.overridden_mode, Some("map".to_string()));
        assert_eq!(result.reason, Some("overlay-set-view"));
    }
}
