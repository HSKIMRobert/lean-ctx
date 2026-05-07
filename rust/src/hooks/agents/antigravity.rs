use super::super::{mcp_server_quiet_mode, resolve_binary_path, write_file};

pub(crate) fn install_antigravity_hook() {
    let Some(home) = crate::core::home::resolve_home_dir() else {
        tracing::error!("Cannot resolve home directory");
        return;
    };

    install_antigravity_mcp_config(&home);
    install_antigravity_gemini_hooks(&home);
}

fn install_antigravity_mcp_config(home: &std::path::Path) {
    let binary = resolve_binary_path();
    let config_path = home
        .join(".gemini")
        .join("antigravity")
        .join("mcp_config.json");

    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let existing = if config_path.exists() {
        std::fs::read_to_string(&config_path).unwrap_or_default()
    } else {
        String::new()
    };

    let already_configured = existing.contains("lean-ctx");
    if already_configured {
        if !mcp_server_quiet_mode() {
            eprintln!(
                "Antigravity MCP: lean-ctx already configured at {}",
                config_path.display()
            );
        }
        return;
    }

    let config = serde_json::json!({
        "mcpServers": {
            "lean-ctx": {
                "command": binary
            }
        }
    });

    if existing.is_empty() || existing.trim() == "{}" || existing.contains("\"mcpServers\": {}") {
        write_file(
            &config_path,
            &serde_json::to_string_pretty(&config).unwrap_or_default(),
        );
    } else if let Ok(mut existing_json) = crate::core::jsonc::parse_jsonc(&existing) {
        if let Some(obj) = existing_json.as_object_mut() {
            let servers = obj
                .entry("mcpServers")
                .or_insert_with(|| serde_json::json!({}));
            if let Some(servers_obj) = servers.as_object_mut() {
                servers_obj.insert(
                    "lean-ctx".to_string(),
                    serde_json::json!({ "command": binary }),
                );
            }
            write_file(
                &config_path,
                &serde_json::to_string_pretty(&existing_json).unwrap_or_default(),
            );
        }
    }

    if !mcp_server_quiet_mode() {
        eprintln!(
            "Installed Antigravity MCP config at {}",
            config_path.display()
        );
    }
}

fn install_antigravity_gemini_hooks(home: &std::path::Path) {
    let binary = resolve_binary_path();
    let rewrite_cmd = format!("{binary} hook rewrite");
    let redirect_cmd = format!("{binary} hook redirect");

    let settings_path = home.join(".gemini").join("settings.json");
    let settings_content = if settings_path.exists() {
        std::fs::read_to_string(&settings_path).unwrap_or_default()
    } else {
        String::new()
    };

    let has_hooks = settings_content.contains("hook rewrite")
        && settings_content.contains("hook redirect")
        && settings_content.contains("\"matcher\"");

    if has_hooks {
        return;
    }

    let hook_config = serde_json::json!({
        "hooks": {
            "BeforeTool": [
                {
                    "matcher": "shell|execute_command|run_shell_command|run_command",
                    "hooks": [{
                        "type": "command",
                        "command": rewrite_cmd
                    }]
                },
                {
                    "matcher": "read_file|view_file|read_many_files|grep|grep_search|search|list_dir",
                    "hooks": [{
                        "type": "command",
                        "command": redirect_cmd
                    }]
                }
            ]
        }
    });

    if settings_content.is_empty() {
        write_file(
            &settings_path,
            &serde_json::to_string_pretty(&hook_config).unwrap_or_default(),
        );
    } else if let Ok(mut existing) = crate::core::jsonc::parse_jsonc(&settings_content) {
        if let Some(obj) = existing.as_object_mut() {
            obj.insert("hooks".to_string(), hook_config["hooks"].clone());
            write_file(
                &settings_path,
                &serde_json::to_string_pretty(&existing).unwrap_or_default(),
            );
        }
    }

    if !mcp_server_quiet_mode() {
        eprintln!(
            "Installed Gemini/Antigravity hooks at {}",
            settings_path.parent().unwrap_or(&settings_path).display()
        );
    }
}
