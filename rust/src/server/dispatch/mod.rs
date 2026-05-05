mod read_tools;
mod session_tools;
mod shell_tools;
mod utility_tools;

use rmcp::ErrorData;
use serde_json::Value;

use crate::server::helpers::get_str;
use crate::tools::LeanCtxServer;

impl LeanCtxServer {
    pub(super) async fn dispatch_tool(
        &self,
        name: &str,
        args: Option<&serde_json::Map<String, Value>>,
        minimal: bool,
    ) -> Result<String, ErrorData> {
        fn format_rate_limited(
            tool: &str,
            agent_id: &str,
            retry_after_ms: u64,
            args: Option<&serde_json::Map<String, Value>>,
        ) -> String {
            let as_json = get_str(args, "format").as_deref() == Some("json");
            if as_json {
                serde_json::json!({
                    "error": "rate_limited",
                    "tool": tool,
                    "agent_id": agent_id,
                    "retry_after_ms": retry_after_ms,
                })
                .to_string()
            } else {
                format!("[RATE LIMITED] tool={tool} retry_after_ms={retry_after_ms}")
            }
        }

        let agent_id = self
            .agent_id
            .read()
            .await
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        if name != "ctx_call" {
            if let crate::core::a2a::rate_limiter::RateLimitResult::Limited { retry_after_ms } =
                crate::core::a2a::rate_limiter::check_rate_limit(&agent_id, name)
            {
                return Ok(format_rate_limited(name, &agent_id, retry_after_ms, args));
            }
        }

        match name {
            "ctx_call" => {
                let inner = get_str(args, "name")
                    .ok_or_else(|| ErrorData::invalid_params("name is required", None))?;
                if inner == "ctx_call" {
                    return Err(ErrorData::invalid_params(
                        "ctx_call cannot invoke itself",
                        None,
                    ));
                }

                let arg_map = match args.and_then(|m| m.get("arguments")) {
                    None | Some(Value::Null) => None,
                    Some(Value::Object(map)) => Some(map.clone()),
                    Some(_) => {
                        return Err(ErrorData::invalid_params(
                            "arguments must be an object",
                            None,
                        ))
                    }
                };

                if let crate::core::a2a::rate_limiter::RateLimitResult::Limited { retry_after_ms } =
                    crate::core::a2a::rate_limiter::check_rate_limit(&agent_id, &inner)
                {
                    return Ok(format_rate_limited(
                        &inner,
                        &agent_id,
                        retry_after_ms,
                        arg_map.as_ref(),
                    ));
                }

                if inner != "ctx_workflow" {
                    let active = self.workflow.read().await.clone();
                    if let Some(run) = active {
                        if let Some(state) = run.spec.state(&run.current) {
                            if let Some(allowed) = &state.allowed_tools {
                                let ok = allowed.iter().any(|t| t == &inner) || inner == "ctx";
                                if !ok {
                                    let mut shown = allowed.clone();
                                    shown.sort();
                                    shown.truncate(30);
                                    return Ok(format!(
                                        "Tool '{inner}' blocked by workflow '{}' (state: {}). Allowed ({} shown): {}",
                                        run.spec.name,
                                        run.current,
                                        shown.len(),
                                        shown.join(", ")
                                    ));
                                }
                            }
                        }
                    }
                }

                let result = self
                    .dispatch_inner(&inner, arg_map.as_ref(), minimal)
                    .await?;
                self.record_call("ctx_call", 0, 0, Some(inner)).await;
                Ok(result)
            }
            _ => self.dispatch_inner(name, args, minimal).await,
        }
    }

    /// Dispatches a single tool to either the trait-based registry or
    /// the legacy match-cascade. Tools are migrated incrementally;
    /// once all tools live in the registry the legacy arms disappear.
    async fn dispatch_inner(
        &self,
        name: &str,
        args: Option<&serde_json::Map<String, Value>>,
        minimal: bool,
    ) -> Result<String, ErrorData> {
        if let Some(tool) = self.registry.as_ref().and_then(|r| r.get(name)) {
            let empty = serde_json::Map::new();
            let args_map = args.unwrap_or(&empty);
            let project_root = {
                let session = self.session.read().await;
                session.project_root.clone().unwrap_or_default()
            };

            let mut resolved_paths = std::collections::HashMap::new();
            for key in ["path", "project_root", "root"] {
                if let Some(raw) = args_map.get(key).and_then(|v| v.as_str()) {
                    if let Ok(resolved) = self.resolve_path(raw).await {
                        resolved_paths.insert(key.to_string(), resolved);
                    }
                }
            }

            let ctx = crate::server::tool_trait::ToolContext {
                project_root,
                minimal,
                resolved_paths,
            };
            let output = tool.handle(args_map, &ctx)?;

            if let Some(ref path) = output.path {
                self.record_call_with_path(
                    name,
                    output.original_tokens,
                    output.saved_tokens,
                    output.mode,
                    Some(path),
                )
                .await;
            } else {
                self.record_call(
                    name,
                    output.original_tokens,
                    output.saved_tokens,
                    output.mode,
                )
                .await;
            }
            return Ok(output.text);
        }

        match name {
            "ctx_read" | "ctx_multi_read" | "ctx_smart_read" | "ctx_delta" | "ctx_edit"
            | "ctx_fill" => self.dispatch_read_tools(name, args, minimal).await,

            "ctx_shell" | "ctx_search" | "ctx_execute" => {
                self.dispatch_shell_tools(name, args, minimal).await
            }

            "ctx_session" | "ctx_knowledge" | "ctx_agent" | "ctx_share" | "ctx_task"
            | "ctx_handoff" | "ctx_workflow" => {
                self.dispatch_session_tools(name, args, minimal).await
            }

            _ => self.dispatch_utility_tools(name, args, minimal).await,
        }
    }
}
