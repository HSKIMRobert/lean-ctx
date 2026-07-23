#[allow(clippy::wildcard_imports)]
use super::super::*;
use super::dispatch_and_post_process;

impl LeanCtxServer {
    pub(crate) async fn call_tool_guarded(
        &self,
        request: CallToolRequestParams,
    ) -> Result<CallToolResult, ErrorData> {
        self.check_idle_expiry().await;
        self.resolve_roots_once().await;
        elicitation::increment_call();

        let original_name = request.name.as_ref().to_string();
        let (resolved_name, resolved_args) = if original_name == "ctx" {
            let sub = request
                .arguments
                .as_ref()
                .and_then(|a| a.get("tool"))
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string)
                .ok_or_else(|| {
                    ErrorData::invalid_params("'tool' is required for ctx meta-tool", None)
                })?;
            let tool_name = if sub.starts_with("ctx_") {
                sub
            } else {
                format!("ctx_{sub}")
            };
            let mut args = request.arguments.unwrap_or_default();
            args.remove("tool");
            (tool_name, Some(args))
        } else {
            (original_name, request.arguments)
        };
        let name = resolved_name.as_str();
        let args = resolved_args.as_ref();

        if let Some(denied) = Self::guard_role_and_policy(name) {
            return Ok(denied);
        }

        // ctx_call is a meta-dispatcher: the egress DLP and permission-
        // inheritance gates below must inspect the INNER tool + arguments, or
        // the universal invoker becomes a policy bypass (#1008 security pass).
        // Role/rate/workflow gates for the inner tool already run inside the
        // dispatch layer; these two ran only on the wrapper name before.
        let inner_call: Option<(String, Option<serde_json::Map<String, serde_json::Value>>)> =
            if name == "ctx_call" {
                helpers::get_str(args, "name").map(|inner_name| {
                    let inner_args = args
                        .and_then(|m| m.get("arguments"))
                        .and_then(serde_json::Value::as_object)
                        .cloned();
                    (inner_name, inner_args)
                })
            } else {
                None
            };
        let (guard_name, guard_args): (&str, Option<&serde_json::Map<_, _>>) = match &inner_call {
            Some((n, a)) => (n.as_str(), a.as_ref()),
            None => (name, args),
        };

        if let Some(blocked) = Self::guard_egress(guard_name, guard_args) {
            return Ok(blocked);
        }

        if let Some(blocked) = self.guard_workflow(name).await {
            return Ok(blocked);
        }

        // #794: cost cap guard — block tool calls when session cost exceeds the
        // configured limit. ctx_session is exempt so the agent can inspect
        // budget status and override the cap.
        if name != "ctx_session"
            && let Some(cap_msg) =
                crate::core::budget_tracker::BudgetTracker::global().cost_cap_message()
        {
            return Ok(CallToolResult::error(vec![ContentBlock::text(cap_msg)]));
        }

        // #990: determine machine-readability *before* the once-per-session
        // decorations below. A machine-readable invocation (e.g. ctx_outline
        // format=json) must reach the client byte-exact and parseable, so every
        // prose decoration and terse compression is suppressed and the pure
        // pre-decoration body is restored at the end (see the `machine_readable`
        // guard near the end of this function). Computing it here — not after
        // dispatch — means such a call also never *consumes* a latched
        // once-per-session flag (auto-context briefing, rules tip) whose prose
        // we would then discard, so those surface on the next human-facing call.
        //
        // `ctx_call` is a meta-dispatcher: the contract belongs to its *inner*
        // tool + inner arguments, not to ctx_call itself. Unwrap one level so
        // JSON reached via the lazy `ctx_call` path (the default advertised
        // surface, where ctx_outline is not a top-level tool) is just as
        // byte-exact as a direct call. This also covers JSON error envelopes
        // from the early rate-limit path, which the first-call auto-context
        // briefing would otherwise corrupt.
        let (mr_name, mr_args): (
            Option<String>,
            Option<&serde_json::Map<String, serde_json::Value>>,
        ) = if name == "ctx_call" {
            (
                helpers::get_str(args, "name"),
                args.and_then(|m| m.get("arguments"))
                    .and_then(serde_json::Value::as_object),
            )
        } else {
            (Some(name.to_string()), args)
        };
        let machine_readable = mr_name
            .as_deref()
            .and_then(|n| self.registry.as_ref().and_then(|r| r.get_arc(n)))
            .is_some_and(|tool| tool.produces_machine_readable(mr_args));

        // Skip the session wake-up briefing for machine-readable calls: the
        // pre-hook latches `session_initialized` via compare-exchange, so calling
        // it here would burn the once-per-session slot for a briefing we then
        // throw away. Deferring keeps the briefing intact for the next call.
        let auto_context = if machine_readable {
            None
        } else {
            let task = {
                let session = self.session.read().await;
                session.task.as_ref().map(|t| t.description.clone())
            };
            let project_root = {
                let session = self.session.read().await;
                session.project_root.clone()
            };
            let cache_timeout =
                tokio::time::timeout(std::time::Duration::from_secs(5), self.cache.write()).await;
            if let Ok(mut cache) = cache_timeout {
                crate::tools::autonomy::session_lifecycle_pre_hook(
                    &self.autonomy,
                    name,
                    &mut cache,
                    task.as_deref(),
                    project_root.as_deref(),
                    CrpMode::effective(),
                )
            } else {
                tracing::warn!("pre-dispatch: cache write-lock timeout (5s), skipping autonomy");
                None
            }
        };

        let args_fp = args
            .map(|a| {
                crate::core::loop_detection::LoopDetector::fingerprint(&serde_json::Value::Object(
                    a.clone(),
                ))
            })
            .unwrap_or_default();
        let throttle_result = {
            let fp = &args_fp;
            let detector_timeout = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                self.loop_detector.write(),
            )
            .await;
            if let Ok(mut detector) = detector_timeout {
                let is_search = crate::core::loop_detection::LoopDetector::is_search_tool(name);
                let is_search_shell = name == "ctx_shell" && {
                    let cmd = args
                        .as_ref()
                        .and_then(|a| a.get("command"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    crate::core::loop_detection::LoopDetector::is_search_shell_command(cmd)
                };

                if is_search || is_search_shell {
                    let search_pattern = args.and_then(|a| {
                        a.get("pattern")
                            .or_else(|| a.get("query"))
                            .and_then(|v| v.as_str())
                    });
                    let shell_pattern = if is_search_shell {
                        args.and_then(|a| a.get("command"))
                            .and_then(|v| v.as_str())
                            .and_then(helpers::extract_search_pattern_from_command)
                    } else {
                        None
                    };
                    let pat = search_pattern.or(shell_pattern.as_deref());
                    detector.record_search(name, fp, pat)
                } else {
                    detector.record_call(name, fp)
                }
            } else {
                tracing::warn!("pre-dispatch: loop_detector write-lock timeout (3s), skipping");
                crate::core::loop_detection::ThrottleResult::default()
            }
        };

        if throttle_result.level == crate::core::loop_detection::ThrottleLevel::Blocked {
            let msg = throttle_result.message.unwrap_or_default();
            return Ok(CallToolResult::success(vec![ContentBlock::text(msg)]));
        }

        let throttle_warning =
            if throttle_result.level == crate::core::loop_detection::ThrottleLevel::Reduced {
                throttle_result.message.clone()
            } else {
                None
            };

        let config = crate::core::config::Config::load_arc();
        let minimal = config.minimal_overhead_effective();

        // IDE permission inheritance: when enabled, mirror the host IDE's
        // bash/read/edit/grep permission rules onto the matching lean-ctx tool so
        // e.g. `ctx_shell` honors a `rm *: ask` rule instead of bypassing it.
        // Gated on the cheap effective() check so the default (off) pays no lock
        // cost on the hot path. Checks the ctx_call-unwrapped inner tool (#1008)
        // so the invoker cannot side-step an IDE deny.
        if config.permission_inheritance_effective()
            == crate::core::config::PermissionInheritance::On
        {
            let client_name = self.client_name.read().await.clone();
            let project_root = self.session.read().await.project_root.clone();
            let perm = permission_inheritance::check(
                &client_name,
                guard_name,
                guard_args,
                project_root.as_deref(),
                &config,
            );
            if let Some(blocked) = permission_inheritance::into_call_tool_result(&perm) {
                tracing::warn!(tool = guard_name, "held back by IDE permission inheritance");
                return Ok(blocked);
            }
        }

        if let Some(msg) = post_process::budget_exhausted_message(name) {
            tracing::warn!(tool = name, "{msg}");
            return Ok(CallToolResult::success(vec![ContentBlock::text(msg)]));
        }

        if is_shell_tool_name(name) {
            crate::core::budget_tracker::BudgetTracker::global().record_shell();
        }

        dispatch_and_post_process(
            self,
            name,
            args,
            minimal,
            config,
            machine_readable,
            auto_context,
            throttle_warning,
            args_fp,
        )
        .await
    }
}
