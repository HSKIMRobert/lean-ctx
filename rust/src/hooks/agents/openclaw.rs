use super::super::resolve_binary_path;

pub(crate) fn install_openclaw_hook() {
    let binary = resolve_binary_path();
    let home = crate::core::home::resolve_home_dir().unwrap_or_default();
    let config_path = home.join(".openclaw/openclaw.json");
    let display_path = "~/.openclaw/openclaw.json";

    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let data_dir = crate::core::data_dir::lean_ctx_data_dir()
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_default();
    let desired = serde_json::json!({
        "command": binary,
        "env": { "LEAN_CTX_DATA_DIR": data_dir }
    });

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        if let Ok(mut json) = crate::core::jsonc::parse_jsonc(&content) {
            if let Some(root) = json.as_object_mut() {
                let mcp = root.entry("mcp").or_insert_with(|| serde_json::json!({}));
                if let Some(mcp_obj) = mcp.as_object_mut() {
                    let servers = mcp_obj
                        .entry("servers")
                        .or_insert_with(|| serde_json::json!({}));
                    if let Some(servers_obj) = servers.as_object_mut() {
                        if servers_obj.get("lean-ctx") == Some(&desired) {
                            if !super::super::mcp_server_quiet_mode() {
                                eprintln!("OpenClaw MCP already configured at {display_path}");
                            }
                            return;
                        }
                        servers_obj.insert("lean-ctx".to_string(), desired.clone());
                    }
                }
                if let Ok(formatted) = serde_json::to_string_pretty(&json) {
                    let _ = std::fs::write(&config_path, formatted);
                    if !super::super::mcp_server_quiet_mode() {
                        eprintln!("  \x1b[32m✓\x1b[0m OpenClaw MCP configured at {display_path}");
                    }
                    return;
                }
            }
        }
    }

    let content = serde_json::to_string_pretty(&serde_json::json!({
        "mcp": {
            "servers": {
                "lean-ctx": desired
            }
        }
    }));

    if let Ok(json_str) = content {
        let _ = std::fs::write(&config_path, json_str);
        if !super::super::mcp_server_quiet_mode() {
            eprintln!("  \x1b[32m✓\x1b[0m OpenClaw MCP configured at {display_path}");
        }
    } else {
        tracing::error!("Failed to configure OpenClaw");
    }
}
