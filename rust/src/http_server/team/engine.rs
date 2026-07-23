#[allow(clippy::wildcard_imports)]
use super::*;

#[derive(Clone)]
pub(super) struct TeamCtxServer {
    pub(super) default_workspace_id: String,
    pub(super) roots: Arc<HashMap<String, String>>,
}

impl TeamCtxServer {
    pub(super) fn default_root(&self) -> &str {
        self.roots
            .get(&self.default_workspace_id)
            .expect("default workspace root")
    }

    fn rewrite_dot_paths(args: &mut Map<String, Value>, root: &str) {
        for k in ["path", "target_directory", "targetDirectory"] {
            let Some(Value::String(s)) = args.get(k) else {
                continue;
            };
            let t = s.trim();
            if t.is_empty() || t == "." {
                args.insert(k.to_string(), Value::String(root.to_string()));
            }
        }
    }

    fn pick_workspace(
        &self,
        args: &mut Map<String, Value>,
    ) -> std::result::Result<(String, String), rmcp::ErrorData> {
        let ws = args
            .get(WORKSPACE_ARG_KEY)
            .and_then(|v| v.as_str())
            .unwrap_or(self.default_workspace_id.as_str())
            .to_string();
        args.remove(WORKSPACE_ARG_KEY);

        let root = self
            .roots
            .get(&ws)
            .cloned()
            .ok_or_else(|| rmcp::ErrorData::invalid_params("unknown workspaceId", None))?;
        Self::rewrite_dot_paths(args, &root);
        Ok((ws, root))
    }

    fn make_server(&self, workspace_id: &str, channel_id: &str) -> LeanCtxServer {
        let root = self
            .roots
            .get(workspace_id)
            .cloned()
            .unwrap_or_else(|| self.default_root().to_string());
        LeanCtxServer::new_shared_with_context(&root, workspace_id, channel_id)
    }
}

impl ServerHandler for TeamCtxServer {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        let s = self.make_server(&self.default_workspace_id, "default");
        <LeanCtxServer as ServerHandler>::get_info(&s)
    }

    async fn initialize(
        &self,
        request: rmcp::model::InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> std::result::Result<rmcp::model::InitializeResult, rmcp::ErrorData> {
        let s = self.make_server(&self.default_workspace_id, "default");
        <LeanCtxServer as ServerHandler>::initialize(&s, request, context).await
    }

    async fn list_tools(
        &self,
        request: Option<rmcp::model::PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> std::result::Result<rmcp::model::ListToolsResult, rmcp::ErrorData> {
        let s = self.make_server(&self.default_workspace_id, "default");
        <LeanCtxServer as ServerHandler>::list_tools(&s, request, context).await
    }

    async fn call_tool(
        &self,
        mut request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let mut args = request.arguments.take().unwrap_or_default();
        let (ws, root) = self.pick_workspace(&mut args)?;
        let channel = args
            .get(CHANNEL_ARG_KEY)
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        args.remove(CHANNEL_ARG_KEY);
        // Per-call agent identity (enterprise#28): the per-call server instance
        // starts with no registered agent, so identity-dependent tools
        // (ctx_share, ctx_agent post/read) receive it via argument — on the
        // REST surface it is the authenticated token id, injected server-side.
        let agent = args
            .get(AGENT_ARG_KEY)
            .and_then(|v| v.as_str())
            .map(str::to_string);
        args.remove(AGENT_ARG_KEY);
        // Re-apply dot path rewriting against the resolved root.
        Self::rewrite_dot_paths(&mut args, &root);
        request.arguments = Some(args);
        let s = LeanCtxServer::new_shared_with_context(&root, &ws, &channel);
        if let Some(agent) = agent {
            *s.agent_id.write().await = Some(agent);
        }
        <LeanCtxServer as ServerHandler>::call_tool(&s, request, context).await
    }
}

pub(super) struct TeamContextEngine {
    pub(super) server: TeamCtxServer,
    next_id: AtomicI64,
}

impl TeamContextEngine {
    pub(super) fn new(server: TeamCtxServer) -> Self {
        Self {
            server,
            next_id: AtomicI64::new(1),
        }
    }

    pub(super) fn manifest_value() -> Value {
        crate::core::mcp_manifest::manifest_value()
    }

    pub(super) async fn call_tool_value(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<Value> {
        let result = self.call_tool_result(name, arguments).await?;
        serde_json::to_value(result).map_err(|e| anyhow!("serialize CallToolResult: {e}"))
    }

    async fn call_tool_result(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let req_id = NumberOrString::Number(id);

        let args_obj: Map<String, Value> = match arguments {
            None => Map::new(),
            Some(Value::Object(m)) => m,
            Some(other) => {
                return Err(anyhow!(
                    "tool arguments must be a JSON object (got {other})"
                ));
            }
        };

        let params = CallToolRequestParams::new(name.to_string()).with_arguments(args_obj);
        let call: CallToolRequest = CallToolRequest::new(params);
        let client_req = ClientRequest::CallToolRequest(call);
        let msg = ClientJsonRpcMessage::Request(JsonRpcRequest::new(req_id, client_req));

        let (transport, mut rx) = OneshotTransport::<RoleServer>::new(msg);
        let service = serve_directly(self.server.clone(), transport, None);
        tokio::spawn(async move {
            let _ = service.waiting().await;
        });

        let Some(server_msg) = rx.recv().await else {
            return Err(anyhow!("no response from tool call"));
        };

        match server_msg {
            ServerJsonRpcMessage::Response(r) => match r.result {
                ServerResult::CallToolResult(result) => Ok(result),
                other => Err(anyhow!("unexpected server result: {other:?}")),
            },
            ServerJsonRpcMessage::Error(e) => Err(anyhow!("{e:?}")).context("tool call error"),
            ServerJsonRpcMessage::Notification(_) => Err(anyhow!("unexpected notification")),
            ServerJsonRpcMessage::Request(_) => Err(anyhow!("unexpected request")),
        }
    }
}
