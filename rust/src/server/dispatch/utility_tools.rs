use rmcp::ErrorData;
use serde_json::Value;

use crate::server::helpers::{get_bool, get_int, get_str, get_str_array};
use crate::tools::LeanCtxServer;

impl LeanCtxServer {
    pub(crate) async fn dispatch_utility_tools(
        &self,
        name: &str,
        args: Option<&serde_json::Map<String, Value>>,
        _minimal: bool,
    ) -> Result<String, ErrorData> {
        Ok(match name {
            "ctx_compress" => {
                let include_sigs = get_bool(args, "include_signatures").unwrap_or(true);
                let cache = self.cache.read().await;
                let result = crate::tools::ctx_compress::handle(
                    &cache,
                    include_sigs,
                    crate::tools::CrpMode::effective(),
                );
                drop(cache);
                self.record_call("ctx_compress", 0, 0, None).await;
                result
            }
            "ctx_metrics" => {
                let cache = self.cache.read().await;
                let calls = self.tool_calls.read().await;
                let mut result = crate::tools::ctx_metrics::handle(
                    &cache,
                    &calls,
                    crate::tools::CrpMode::effective(),
                );
                drop(cache);
                drop(calls);
                let stats = self.pipeline_stats.read().await;
                if stats.runs > 0 {
                    result.push_str("\n\n--- PIPELINE METRICS ---\n");
                    result.push_str(&stats.format_summary());
                }
                drop(stats);
                let (ts_hits, regex_hits) = crate::core::signatures::signature_backend_stats();
                if ts_hits + regex_hits > 0 {
                    result.push_str("\n--- SIGNATURE BACKEND ---\n");
                    result.push_str(&format!(
                        "tree-sitter: {} | regex fallback: {} | ratio: {:.0}%\n",
                        ts_hits,
                        regex_hits,
                        if ts_hits + regex_hits > 0 {
                            ts_hits as f64 / (ts_hits + regex_hits) as f64 * 100.0
                        } else {
                            0.0
                        }
                    ));
                }
                self.record_call("ctx_metrics", 0, 0, None).await;
                result
            }
            "ctx_dedup" => {
                let action = get_str(args, "action").unwrap_or_default();
                if action == "apply" {
                    let mut cache = self.cache.write().await;
                    let result = crate::tools::ctx_dedup::handle_action(&mut cache, &action);
                    drop(cache);
                    self.record_call("ctx_dedup", 0, 0, None).await;
                    result
                } else {
                    let cache = self.cache.read().await;
                    let result = crate::tools::ctx_dedup::handle(&cache);
                    drop(cache);
                    self.record_call("ctx_dedup", 0, 0, None).await;
                    result
                }
            }
            "ctx_intent" => {
                let query = get_str(args, "query")
                    .ok_or_else(|| ErrorData::invalid_params("query is required", None))?;
                let root = get_str(args, "project_root").unwrap_or_else(|| ".".to_string());
                let format = get_str(args, "format");
                let mut cache = self.cache.write().await;
                let output = crate::tools::ctx_intent::handle(
                    &mut cache,
                    &query,
                    &root,
                    crate::tools::CrpMode::effective(),
                    format.as_deref(),
                );
                drop(cache);
                {
                    let mut session = self.session.write().await;
                    session.set_task(&query, Some("intent"));
                }
                self.record_call("ctx_intent", 0, 0, Some("semantic".to_string()))
                    .await;
                output
            }
            "ctx_context" => {
                let cache = self.cache.read().await;
                let turn = self.call_count.load(std::sync::atomic::Ordering::Relaxed);
                let result = crate::tools::ctx_context::handle_status(
                    &cache,
                    turn,
                    crate::tools::CrpMode::effective(),
                );
                drop(cache);
                self.record_call("ctx_context", 0, 0, None).await;
                result
            }
            "ctx_graph" => {
                let action = get_str(args, "action")
                    .ok_or_else(|| ErrorData::invalid_params("action is required", None))?;
                let path = match get_str(args, "path") {
                    Some(p) if action == "diagram" => Some(p),
                    Some(p) => Some(
                        self.resolve_path(&p)
                            .await
                            .map_err(|e| ErrorData::invalid_params(e, None))?,
                    ),
                    None => None,
                };
                let root = self
                    .resolve_path(&get_str(args, "project_root").unwrap_or_else(|| ".".to_string()))
                    .await
                    .map_err(|e| ErrorData::invalid_params(e, None))?;
                let depth = get_int(args, "depth").map(|d| d as usize);
                let kind = get_str(args, "kind");
                let crp_mode = crate::tools::CrpMode::effective();
                let action_for_record = action.clone();
                let mut cache = self.cache.write().await;
                let result = crate::tools::ctx_graph::handle(
                    &action,
                    path.as_deref(),
                    &root,
                    &mut cache,
                    crp_mode,
                    depth,
                    kind.as_deref(),
                );
                drop(cache);
                self.record_call("ctx_graph", 0, 0, Some(action_for_record))
                    .await;
                result
            }
            "ctx_proof" => {
                let action = get_str(args, "action")
                    .ok_or_else(|| ErrorData::invalid_params("action is required", None))?;
                if action != "export" {
                    return Err(ErrorData::invalid_params(
                        "unsupported action (expected: export)",
                        None,
                    ));
                }
                let root = self
                    .resolve_path(&get_str(args, "project_root").unwrap_or_else(|| ".".to_string()))
                    .await
                    .map_err(|e| ErrorData::invalid_params(e, None))?;
                let format = get_str(args, "format");
                let write = get_bool(args, "write").unwrap_or(true);
                let filename = get_str(args, "filename");
                let max_evidence = get_int(args, "max_evidence").map(|v| v as usize);
                let max_ledger_files = get_int(args, "max_ledger_files").map(|v| v as usize);

                let sources = crate::core::context_proof::ProofSources {
                    project_root: Some(root.clone()),
                    session: Some(self.session.read().await.clone()),
                    pipeline: Some(self.pipeline_stats.read().await.clone()),
                    ledger: Some(self.ledger.read().await.clone()),
                };

                let out = crate::tools::ctx_proof::handle_export(
                    &root,
                    format.as_deref(),
                    write,
                    filename.as_deref(),
                    max_evidence,
                    max_ledger_files,
                    sources,
                )
                .map_err(|e| ErrorData::invalid_params(e, None))?;
                self.record_call_with_path("ctx_proof", 0, 0, Some(action), Some(&root))
                    .await;
                out
            }
            "ctx_cache" => {
                let action = get_str(args, "action")
                    .ok_or_else(|| ErrorData::invalid_params("action is required", None))?;
                let mut cache = self.cache.write().await;
                let result = match action.as_str() {
                    "status" => {
                        let entries = cache.get_all_entries();
                        if entries.is_empty() {
                            "Cache empty — no files tracked.".to_string()
                        } else {
                            let mut lines = vec![format!("Cache: {} file(s)", entries.len())];
                            for (path, entry) in &entries {
                                let fref = cache
                                    .file_ref_map()
                                    .get(*path)
                                    .map_or("F?", std::string::String::as_str);
                                lines.push(format!(
                                    "  {fref}={} [{}L, {}t, read {}x]",
                                    crate::core::protocol::shorten_path(path),
                                    entry.line_count,
                                    entry.original_tokens,
                                    entry.read_count
                                ));
                            }
                            lines.join("\n")
                        }
                    }
                    "clear" => {
                        let count = cache.clear();
                        format!("Cache cleared — {count} file(s) removed. Next ctx_read will return full content.")
                    }
                    "invalidate" => {
                        let path = match get_str(args, "path") {
                            Some(p) => self
                                .resolve_path(&p)
                                .await
                                .map_err(|e| ErrorData::invalid_params(e, None))?,
                            None => {
                                return Err(ErrorData::invalid_params(
                                    "path is required for invalidate",
                                    None,
                                ))
                            }
                        };
                        if cache.invalidate(&path) {
                            format!(
                                "Invalidated cache for {}. Next ctx_read will return full content.",
                                crate::core::protocol::shorten_path(&path)
                            )
                        } else {
                            format!(
                                "{} was not in cache.",
                                crate::core::protocol::shorten_path(&path)
                            )
                        }
                    }
                    _ => "Unknown action. Use: status, clear, invalidate".to_string(),
                };
                drop(cache);
                self.record_call("ctx_cache", 0, 0, Some(action)).await;
                result
            }
            "ctx_overview" => {
                let task = get_str(args, "task");
                let resolved_path = if let Some(p) = get_str(args, "path") {
                    Some(
                        self.resolve_path(&p)
                            .await
                            .map_err(|e| ErrorData::invalid_params(e, None))?,
                    )
                } else {
                    let session = self.session.read().await;
                    session.project_root.clone()
                };
                let cache = self.cache.read().await;
                let crp_mode = crate::tools::CrpMode::effective();
                let result = crate::tools::ctx_overview::handle(
                    &cache,
                    task.as_deref(),
                    resolved_path.as_deref(),
                    crp_mode,
                );
                drop(cache);
                self.record_call("ctx_overview", 0, 0, Some("overview".to_string()))
                    .await;
                result
            }
            "ctx_preload" => {
                let task = get_str(args, "task").unwrap_or_default();
                let resolved_path = if let Some(p) = get_str(args, "path") {
                    Some(
                        self.resolve_path(&p)
                            .await
                            .map_err(|e| ErrorData::invalid_params(e, None))?,
                    )
                } else {
                    let session = self.session.read().await;
                    session.project_root.clone()
                };
                let mut cache = self.cache.write().await;
                let mut result = crate::tools::ctx_preload::handle(
                    &mut cache,
                    &task,
                    resolved_path.as_deref(),
                    crate::tools::CrpMode::effective(),
                );
                drop(cache);

                {
                    let mut session = self.session.write().await;
                    if session.active_structured_intent.is_none()
                        || session
                            .active_structured_intent
                            .as_ref()
                            .is_none_or(|i| i.confidence < 0.6)
                    {
                        session.set_task(&task, Some("preload"));
                    }
                }

                let session = self.session.read().await;
                if let Some(ref intent) = session.active_structured_intent {
                    let ledger = self.ledger.read().await;
                    if !ledger.entries.is_empty() {
                        let known: Vec<String> = session
                            .files_touched
                            .iter()
                            .map(|f| f.path.clone())
                            .collect();
                        let deficit =
                            crate::core::context_deficit::detect_deficit(&ledger, intent, &known);
                        if !deficit.suggested_files.is_empty() {
                            result.push_str("\n\n--- SUGGESTED FILES ---");
                            for s in &deficit.suggested_files {
                                result.push_str(&format!(
                                    "\n  {} ({:?}, ~{} tok, mode: {})",
                                    s.path, s.reason, s.estimated_tokens, s.recommended_mode
                                ));
                            }
                        }

                        let pressure = ledger.pressure();
                        if pressure.utilization > 0.7 {
                            let plan = ledger.reinjection_plan(intent, 0.6);
                            if !plan.actions.is_empty() {
                                result.push_str("\n\n--- REINJECTION PLAN ---");
                                result.push_str(&format!(
                                    "\n  Context pressure: {:.0}% -> target: 60%",
                                    pressure.utilization * 100.0
                                ));
                                for a in &plan.actions {
                                    result.push_str(&format!(
                                        "\n  {} : {} -> {} (frees ~{} tokens)",
                                        a.path, a.current_mode, a.new_mode, a.tokens_freed
                                    ));
                                }
                                result.push_str(&format!(
                                    "\n  Total freeable: {} tokens",
                                    plan.total_tokens_freed
                                ));
                            }
                        }
                    }
                }
                drop(session);

                self.record_call("ctx_preload", 0, 0, Some("preload".to_string()))
                    .await;
                result
            }
            "ctx_prefetch" => {
                let root = if let Some(r) = get_str(args, "root") {
                    self.resolve_path(&r)
                        .await
                        .map_err(|e| ErrorData::invalid_params(e, None))?
                } else {
                    let session = self.session.read().await;
                    session
                        .project_root
                        .clone()
                        .unwrap_or_else(|| ".".to_string())
                };
                let task = get_str(args, "task");
                let changed_files = get_str_array(args, "changed_files");
                let budget_tokens =
                    get_int(args, "budget_tokens").map_or(3000, |n| n.max(0) as usize);
                let max_files = get_int(args, "max_files").map(|n| n.max(1) as usize);

                let mut resolved_changed: Option<Vec<String>> = None;
                if let Some(files) = changed_files {
                    let mut v = Vec::with_capacity(files.len());
                    for p in files {
                        v.push(
                            self.resolve_path(&p)
                                .await
                                .map_err(|e| ErrorData::invalid_params(e, None))?,
                        );
                    }
                    resolved_changed = Some(v);
                }

                let mut cache = self.cache.write().await;
                let result = crate::tools::ctx_prefetch::handle(
                    &mut cache,
                    &root,
                    task.as_deref(),
                    resolved_changed.as_deref(),
                    budget_tokens,
                    max_files,
                    crate::tools::CrpMode::effective(),
                );
                drop(cache);
                self.record_call("ctx_prefetch", 0, 0, Some("prefetch".to_string()))
                    .await;
                result
            }
            "ctx_semantic_search" => {
                let query = get_str(args, "query")
                    .ok_or_else(|| ErrorData::invalid_params("query is required", None))?;
                let path = self
                    .resolve_path(&get_str(args, "path").unwrap_or_else(|| ".".to_string()))
                    .await
                    .map_err(|e| ErrorData::invalid_params(e, None))?;
                let top_k = get_int(args, "top_k").unwrap_or(10) as usize;
                let action = get_str(args, "action").unwrap_or_default();
                let mode = get_str(args, "mode");
                let languages = get_str_array(args, "languages");
                let path_glob = get_str(args, "path_glob");
                let workspace = get_bool(args, "workspace").unwrap_or(false);
                let artifacts = get_bool(args, "artifacts").unwrap_or(false);

                #[cfg(feature = "qdrant")]
                {
                    let mode_effective = mode
                        .as_deref()
                        .unwrap_or("hybrid")
                        .trim()
                        .to_ascii_lowercase();
                    if action != "reindex"
                        && !artifacts
                        && matches!(mode_effective.as_str(), "dense" | "hybrid")
                        && matches!(
                            crate::core::dense_backend::DenseBackendKind::try_from_env(),
                            Ok(crate::core::dense_backend::DenseBackendKind::Qdrant)
                        )
                    {
                        let value = format!(
                            "tool=ctx_semantic_search mode={mode_effective} workspace={workspace}"
                        );
                        let mut session = self.session.write().await;
                        session.record_manual_evidence("remote:qdrant_query", Some(&value));
                    }
                }

                let result = if action == "reindex" {
                    if artifacts {
                        crate::tools::ctx_semantic_search::handle_reindex_artifacts(
                            &path, workspace,
                        )
                    } else {
                        crate::tools::ctx_semantic_search::handle_reindex(&path)
                    }
                } else {
                    crate::tools::ctx_semantic_search::handle(
                        &query,
                        &path,
                        top_k,
                        crate::tools::CrpMode::effective(),
                        languages.as_deref(),
                        path_glob.as_deref(),
                        mode.as_deref(),
                        Some(workspace),
                        Some(artifacts),
                    )
                };
                self.record_call("ctx_semantic_search", 0, 0, Some("semantic".to_string()))
                    .await;
                let repeat_hint = if action == "reindex" {
                    String::new()
                } else {
                    self.autonomy
                        .track_search(&query, &path)
                        .map(|h| format!("\n{h}"))
                        .unwrap_or_default()
                };
                format!("{result}{repeat_hint}")
            }
            "ctx_feedback" => {
                let action = get_str(args, "action").unwrap_or_else(|| "report".to_string());
                let limit = get_int(args, "limit").map_or(500, |n| n.max(1) as usize);
                match action.as_str() {
                    "record" => {
                        let current_agent_id = { self.agent_id.read().await.clone() };
                        let agent_id = get_str(args, "agent_id").or(current_agent_id);
                        let agent_id = agent_id.ok_or_else(|| {
                            ErrorData::invalid_params(
                                "agent_id is required (or register an agent via project_root detection first)",
                                None,
                            )
                        })?;

                        let (ctx_read_last_mode, ctx_read_modes) = {
                            let calls = self.tool_calls.read().await;
                            let mut last: Option<String> = None;
                            let mut modes: std::collections::BTreeMap<String, u64> =
                                std::collections::BTreeMap::new();
                            for rec in calls.iter().rev().take(50) {
                                if rec.tool != "ctx_read" {
                                    continue;
                                }
                                if let Some(m) = rec.mode.as_ref() {
                                    *modes.entry(m.clone()).or_insert(0) += 1;
                                    if last.is_none() {
                                        last = Some(m.clone());
                                    }
                                }
                            }
                            (last, if modes.is_empty() { None } else { Some(modes) })
                        };

                        let llm_input_tokens =
                            get_int(args, "llm_input_tokens").ok_or_else(|| {
                                ErrorData::invalid_params("llm_input_tokens is required", None)
                            })?;
                        let llm_output_tokens =
                            get_int(args, "llm_output_tokens").ok_or_else(|| {
                                ErrorData::invalid_params("llm_output_tokens is required", None)
                            })?;
                        if llm_input_tokens <= 0 || llm_output_tokens <= 0 {
                            return Err(ErrorData::invalid_params(
                                "llm_input_tokens and llm_output_tokens must be > 0",
                                None,
                            ));
                        }

                        let ev = crate::core::llm_feedback::LlmFeedbackEvent {
                            agent_id,
                            intent: get_str(args, "intent"),
                            model: get_str(args, "model"),
                            llm_input_tokens: llm_input_tokens as u64,
                            llm_output_tokens: llm_output_tokens as u64,
                            latency_ms: get_int(args, "latency_ms").map(|n| n.max(0) as u64),
                            note: get_str(args, "note"),
                            ctx_read_last_mode,
                            ctx_read_modes,
                            timestamp: chrono::Local::now().to_rfc3339(),
                        };
                        let result = crate::tools::ctx_feedback::record(&ev)
                            .unwrap_or_else(|e| format!("Error recording feedback: {e}"));
                        self.record_call("ctx_feedback", 0, 0, Some(action)).await;
                        result
                    }
                    "status" => {
                        let result = crate::tools::ctx_feedback::status();
                        self.record_call("ctx_feedback", 0, 0, Some(action)).await;
                        result
                    }
                    "json" => {
                        let result = crate::tools::ctx_feedback::json(limit);
                        self.record_call("ctx_feedback", 0, 0, Some(action)).await;
                        result
                    }
                    "reset" => {
                        let result = crate::tools::ctx_feedback::reset();
                        self.record_call("ctx_feedback", 0, 0, Some(action)).await;
                        result
                    }
                    _ => {
                        let result = crate::tools::ctx_feedback::report(limit);
                        self.record_call("ctx_feedback", 0, 0, Some(action)).await;
                        result
                    }
                }
            }

            _ => {
                return Err(ErrorData::invalid_params(
                    format!("Unknown tool: {name}"),
                    None,
                ));
            }
        })
    }
}
