use rmcp::ErrorData;
use serde_json::Value;

use crate::server::helpers::{get_bool, get_int, get_str, get_str_array};
use crate::tools::LeanCtxServer;

fn auto_degrade_read_mode(mode: &str) -> String {
    use crate::core::degradation_policy::DegradationVerdictV1;
    let profile = crate::core::profiles::active_profile();
    if !profile.degradation.enforce_effective() {
        return mode.to_string();
    }
    let policy = crate::core::degradation_policy::evaluate_v1_for_tool("ctx_read", None);
    match policy.decision.verdict {
        DegradationVerdictV1::Ok => mode.to_string(),
        DegradationVerdictV1::Warn => match mode {
            "full" => "map".to_string(),
            other => other.to_string(),
        },
        DegradationVerdictV1::Throttle => match mode {
            "full" | "map" => "signatures".to_string(),
            other => other.to_string(),
        },
        DegradationVerdictV1::Block => "signatures".to_string(),
    }
}

impl LeanCtxServer {
    pub(crate) async fn dispatch_read_tools(
        &self,
        name: &str,
        args: Option<&serde_json::Map<String, Value>>,
        minimal: bool,
    ) -> Result<String, ErrorData> {
        Ok(match name {
            "ctx_read" => {
                let path = match get_str(args, "path") {
                    Some(p) => self
                        .resolve_path(&p)
                        .await
                        .map_err(|e| ErrorData::invalid_params(e, None))?,
                    None => return Err(ErrorData::invalid_params("path is required", None)),
                };
                let current_task = {
                    let session = self.session.read().await;
                    session.task.as_ref().map(|t| t.description.clone())
                };
                let task_ref = current_task.as_deref();
                let profile = crate::core::profiles::active_profile();
                let mut mode = if let Some(m) = get_str(args, "mode") {
                    m
                } else if profile.read.default_mode_effective() == "auto" {
                    let cache = self.cache.read().await;
                    crate::tools::ctx_smart_read::select_mode_with_task(&cache, &path, task_ref)
                } else {
                    profile.read.default_mode_effective().to_string()
                };
                let mut fresh = get_bool(args, "fresh").unwrap_or(false);
                let start_line = get_int(args, "start_line");
                if let Some(sl) = start_line {
                    let sl = sl.max(1_i64);
                    mode = format!("lines:{sl}-999999");
                    fresh = true;
                }
                let mode = auto_degrade_read_mode(&mode);
                let stale = self.is_prompt_cache_stale().await;
                let effective_mode = LeanCtxServer::upgrade_mode_if_stale(&mode, stale).to_string();
                if mode.starts_with("lines:") {
                    fresh = true;
                }
                if stale && effective_mode == "full" && !fresh {
                    fresh = true;
                }
                let read_start = std::time::Instant::now();
                let mut cache = self.cache.write().await;
                let read_output = if fresh {
                    crate::tools::ctx_read::handle_fresh_with_task_resolved(
                        &mut cache,
                        &path,
                        &effective_mode,
                        crate::tools::CrpMode::effective(),
                        task_ref,
                    )
                } else {
                    crate::tools::ctx_read::handle_with_task_resolved(
                        &mut cache,
                        &path,
                        &effective_mode,
                        crate::tools::CrpMode::effective(),
                        task_ref,
                    )
                };
                let output = read_output.content;
                let resolved_mode = read_output.resolved_mode;
                let stale_note = if !minimal && effective_mode != mode {
                    format!("[cache stale, {mode}→{effective_mode}]\n")
                } else {
                    String::new()
                };
                let original = cache.get(&path).map_or(0, |e| e.original_tokens);
                let is_cache_hit = output.contains(" cached ");
                let output = format!("{stale_note}{output}");
                let output_tokens = crate::core::tokens::count_tokens(&output);
                let saved = original.saturating_sub(output_tokens);
                let file_ref = cache.file_ref_map().get(&path).cloned();
                drop(cache);
                let mut ensured_root: Option<String> = None;
                {
                    let mut session = self.session.write().await;
                    session.touch_file(&path, file_ref.as_deref(), &resolved_mode, original);
                    if is_cache_hit {
                        session.record_cache_hit();
                    }
                    if session.active_structured_intent.is_none()
                        && session.files_touched.len() >= 2
                    {
                        let touched: Vec<String> = session
                            .files_touched
                            .iter()
                            .map(|f| f.path.clone())
                            .collect();
                        let inferred =
                            crate::core::intent_engine::StructuredIntent::from_file_patterns(
                                &touched,
                            );
                        if inferred.confidence >= 0.4 {
                            session.active_structured_intent = Some(inferred);
                        }
                    }
                    let root_missing = session
                        .project_root
                        .as_deref()
                        .is_none_or(|r| r.trim().is_empty());
                    if root_missing {
                        if let Some(root) = crate::core::protocol::detect_project_root(&path) {
                            session.project_root = Some(root.clone());
                            ensured_root = Some(root.clone());
                            let mut current = self.agent_id.write().await;
                            if current.is_none() {
                                let mut registry =
                                    crate::core::agents::AgentRegistry::load_or_create();
                                registry.cleanup_stale(24);
                                let role = std::env::var("LEAN_CTX_AGENT_ROLE").ok();
                                let id = registry.register("mcp", role.as_deref(), &root);
                                let _ = registry.save();
                                *current = Some(id);
                            }
                        }
                    }
                }
                if let Some(root) = ensured_root.as_deref() {
                    crate::core::index_orchestrator::ensure_all_background(root);
                }
                self.record_call_with_path(
                    "ctx_read",
                    original,
                    saved,
                    Some(resolved_mode.clone()),
                    Some(&path),
                )
                .await;
                crate::core::heatmap::record_file_access(&path, original, saved);
                {
                    let mut ledger = self.ledger.write().await;
                    ledger.record(&path, &resolved_mode, original, output_tokens);
                    ledger.save();
                }
                {
                    let duration = read_start.elapsed();
                    let duration_us = duration.as_micros() as u64;

                    let mut stats = self.pipeline_stats.write().await;
                    stats.record(&[crate::core::pipeline::LayerMetrics::new(
                        crate::core::pipeline::LayerKind::Compression,
                        original,
                        output_tokens,
                        duration_us,
                    )]);
                    stats.save();

                    if let Some(ref ir_lock) = self.context_ir {
                        let client_name = { self.client_name.read().await.clone() };
                        let agent_id = { self.agent_id.read().await.clone() };
                        let mut ir = ir_lock.write().await;
                        ir.record(crate::core::context_ir::RecordIrInput {
                            kind: crate::core::context_ir::ContextIrSourceKindV1::Read,
                            tool: "ctx_read",
                            client_name: if client_name.trim().is_empty() {
                                None
                            } else {
                                Some(client_name)
                            },
                            agent_id,
                            path: Some(&path),
                            command: None,
                            pattern: None,
                            input_tokens: original,
                            output_tokens,
                            duration,
                            content_excerpt: &output,
                        });
                        ir.save();
                    }
                }
                {
                    let sig =
                        crate::core::mode_predictor::FileSignature::from_path(&path, original);
                    let density = if output_tokens > 0 {
                        original as f64 / output_tokens as f64
                    } else {
                        1.0
                    };
                    let outcome = crate::core::mode_predictor::ModeOutcome {
                        mode: resolved_mode.clone(),
                        tokens_in: original,
                        tokens_out: output_tokens,
                        density: density.min(1.0),
                    };
                    let project_root = {
                        let session = self.session.read().await;
                        session
                            .project_root
                            .clone()
                            .unwrap_or_else(|| ".".to_string())
                    };
                    let mut predictor = crate::core::mode_predictor::ModePredictor::new();
                    predictor.set_project_root(&project_root);
                    predictor.record(sig, outcome);
                    predictor.save();

                    let ext = std::path::Path::new(&path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_string();
                    let thresholds = crate::core::adaptive_thresholds::thresholds_for_path(&path);
                    let cache = self.cache.read().await;
                    let stats = cache.get_stats();
                    let feedback_outcome = crate::core::feedback::CompressionOutcome {
                        session_id: format!("{}", std::process::id()),
                        language: ext,
                        entropy_threshold: thresholds.bpe_entropy,
                        jaccard_threshold: thresholds.jaccard,
                        total_turns: stats.total_reads as u32,
                        tokens_saved: saved as u64,
                        tokens_original: original as u64,
                        cache_hits: stats.cache_hits as u32,
                        total_reads: stats.total_reads as u32,
                        task_completed: true,
                        timestamp: chrono::Local::now().to_rfc3339(),
                    };
                    drop(cache);
                    let mut store = crate::core::feedback::FeedbackStore::load();
                    store.project_root = Some(project_root.clone());
                    store.record_outcome(feedback_outcome);
                }
                output
            }
            "ctx_multi_read" => {
                let raw_paths = get_str_array(args, "paths")
                    .ok_or_else(|| ErrorData::invalid_params("paths array is required", None))?;
                let mut paths = Vec::with_capacity(raw_paths.len());
                for p in raw_paths {
                    paths.push(
                        self.resolve_path(&p)
                            .await
                            .map_err(|e| ErrorData::invalid_params(e, None))?,
                    );
                }
                let mode = get_str(args, "mode").unwrap_or_else(|| {
                    let p = crate::core::profiles::active_profile();
                    let dm = p.read.default_mode_effective();
                    if dm == "auto" {
                        "full".to_string()
                    } else {
                        dm.to_string()
                    }
                });
                let current_task = {
                    let session = self.session.read().await;
                    session.task.as_ref().map(|t| t.description.clone())
                };
                let mut cache = self.cache.write().await;
                let output = crate::tools::ctx_multi_read::handle_with_task(
                    &mut cache,
                    &paths,
                    &mode,
                    crate::tools::CrpMode::effective(),
                    current_task.as_deref(),
                );
                let mut total_original: usize = 0;
                for path in &paths {
                    total_original = total_original
                        .saturating_add(cache.get(path).map_or(0, |e| e.original_tokens));
                }
                let tokens = crate::core::tokens::count_tokens(&output);
                drop(cache);
                self.record_call(
                    "ctx_multi_read",
                    total_original,
                    total_original.saturating_sub(tokens),
                    Some(mode),
                )
                .await;
                output
            }
            "ctx_smart_read" => {
                let path = match get_str(args, "path") {
                    Some(p) => self
                        .resolve_path(&p)
                        .await
                        .map_err(|e| ErrorData::invalid_params(e, None))?,
                    None => return Err(ErrorData::invalid_params("path is required", None)),
                };
                let mut cache = self.cache.write().await;
                let output = crate::tools::ctx_smart_read::handle(
                    &mut cache,
                    &path,
                    crate::tools::CrpMode::effective(),
                );
                let original = cache.get(&path).map_or(0, |e| e.original_tokens);
                let tokens = crate::core::tokens::count_tokens(&output);
                drop(cache);
                self.record_call_with_path(
                    "ctx_smart_read",
                    original,
                    original.saturating_sub(tokens),
                    Some("auto".to_string()),
                    Some(&path),
                )
                .await;
                output
            }
            "ctx_delta" => {
                let path = match get_str(args, "path") {
                    Some(p) => self
                        .resolve_path(&p)
                        .await
                        .map_err(|e| ErrorData::invalid_params(e, None))?,
                    None => return Err(ErrorData::invalid_params("path is required", None)),
                };
                let mut cache = self.cache.write().await;
                let output = crate::tools::ctx_delta::handle(&mut cache, &path);
                let original = cache.get(&path).map_or(0, |e| e.original_tokens);
                let tokens = crate::core::tokens::count_tokens(&output);
                drop(cache);
                {
                    let mut session = self.session.write().await;
                    session.mark_modified(&path);
                }
                self.record_call_with_path(
                    "ctx_delta",
                    original,
                    original.saturating_sub(tokens),
                    Some("delta".to_string()),
                    Some(&path),
                )
                .await;
                output
            }
            "ctx_edit" => {
                let path = match get_str(args, "path") {
                    Some(p) => self
                        .resolve_path(&p)
                        .await
                        .map_err(|e| ErrorData::invalid_params(e, None))?,
                    None => return Err(ErrorData::invalid_params("path is required", None)),
                };
                let old_string = get_str(args, "old_string").unwrap_or_default();
                let new_string = get_str(args, "new_string")
                    .ok_or_else(|| ErrorData::invalid_params("new_string is required", None))?;
                let replace_all = args
                    .as_ref()
                    .and_then(|a| a.get("replace_all"))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let create = args
                    .as_ref()
                    .and_then(|a| a.get("create"))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let expected_md5 = get_str(args, "expected_md5");
                let expected_size =
                    get_int(args, "expected_size").and_then(|v| u64::try_from(v).ok());
                let expected_mtime_ms =
                    get_int(args, "expected_mtime_ms").and_then(|v| u64::try_from(v).ok());
                let backup = get_bool(args, "backup").unwrap_or(false);
                let backup_path = match get_str(args, "backup_path") {
                    Some(p) => Some(
                        self.resolve_path(&p)
                            .await
                            .map_err(|e| ErrorData::invalid_params(e, None))?,
                    ),
                    None => None,
                };
                let evidence = get_bool(args, "evidence").unwrap_or(true);
                let diff_max_lines = get_int(args, "diff_max_lines")
                    .and_then(|v| usize::try_from(v.max(0)).ok())
                    .unwrap_or(200);
                let allow_lossy_utf8 = get_bool(args, "allow_lossy_utf8").unwrap_or(false);

                let mut cache = self.cache.write().await;
                let output = crate::tools::ctx_edit::handle(
                    &mut cache,
                    &crate::tools::ctx_edit::EditParams {
                        path: path.clone(),
                        old_string,
                        new_string,
                        replace_all,
                        create,
                        expected_md5,
                        expected_size,
                        expected_mtime_ms,
                        backup,
                        backup_path,
                        evidence,
                        diff_max_lines,
                        allow_lossy_utf8,
                    },
                );
                drop(cache);

                {
                    let mut session = self.session.write().await;
                    session.mark_modified(&path);
                }
                self.record_call_with_path("ctx_edit", 0, 0, None, Some(&path))
                    .await;
                output
            }
            "ctx_fill" => {
                let raw_paths = get_str_array(args, "paths")
                    .ok_or_else(|| ErrorData::invalid_params("paths array is required", None))?;
                let mut paths = Vec::with_capacity(raw_paths.len());
                for p in raw_paths {
                    paths.push(
                        self.resolve_path(&p)
                            .await
                            .map_err(|e| ErrorData::invalid_params(e, None))?,
                    );
                }
                let budget = get_int(args, "budget")
                    .ok_or_else(|| ErrorData::invalid_params("budget is required", None))?
                    as usize;
                let task = get_str(args, "task");
                let mut cache = self.cache.write().await;
                let output = crate::tools::ctx_fill::handle(
                    &mut cache,
                    &paths,
                    budget,
                    crate::tools::CrpMode::effective(),
                    task.as_deref(),
                );
                drop(cache);
                self.record_call("ctx_fill", 0, 0, Some(format!("budget:{budget}")))
                    .await;
                output
            }
            _ => unreachable!("dispatch_read_tools called with unknown tool: {name}"),
        })
    }
}
