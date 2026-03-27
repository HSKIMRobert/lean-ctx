use std::path::PathBuf;

struct EditorTarget {
    name: &'static str,
    agent_key: &'static str,
    config_path: PathBuf,
    detect_path: PathBuf,
    config_type: ConfigType,
}

enum ConfigType {
    McpJson,
    Zed,
    Codex,
    VsCodeMcp,
}

pub fn run_setup() {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Cannot determine home directory");
            std::process::exit(1);
        }
    };

    let binary = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "lean-ctx".to_string());

    println!("\n\x1b[1;32m▶\x1b[0m \x1b[1mlean-ctx setup\x1b[0m\n");

    println!("\x1b[1m1. Installing shell aliases...\x1b[0m");
    crate::cli::cmd_init(&["--global".to_string()]);
    println!();

    println!("\x1b[1m2. Detecting installed editors...\x1b[0m");

    let targets = build_targets(&home, &binary);
    let mut configured: Vec<&str> = Vec::new();
    let mut skipped: Vec<&str> = Vec::new();

    for target in &targets {
        if !target.detect_path.exists() {
            continue;
        }

        let already_configured = target.config_path.exists()
            && std::fs::read_to_string(&target.config_path)
                .map(|c| c.contains("lean-ctx"))
                .unwrap_or(false);

        if already_configured {
            println!(
                "  \x1b[32m✓\x1b[0m {} \x1b[2m(already configured)\x1b[0m",
                target.name
            );
            skipped.push(target.name);
            continue;
        }

        match write_config(target, &binary) {
            Ok(()) => {
                println!(
                    "  \x1b[32m✓\x1b[0m {} \x1b[2m(created {})\x1b[0m",
                    target.name,
                    target.config_path.display()
                );
                configured.push(target.name);
            }
            Err(e) => {
                println!(
                    "  \x1b[33m⚠\x1b[0m {} \x1b[2m(error: {})\x1b[0m",
                    target.name, e
                );
            }
        }
    }

    if configured.is_empty() && skipped.is_empty() {
        println!(
            "  \x1b[33m⚠\x1b[0m No editors detected. \
             Configure manually: https://leanctx.com/docs/getting-started#editor-setup"
        );
    }

    println!();
    println!("\x1b[1m3. Installing agent instructions...\x1b[0m");
    let mut agents_installed = 0;
    for target in &targets {
        if !target.detect_path.exists() || target.agent_key.is_empty() {
            continue;
        }
        crate::hooks::install_agent_hook(target.agent_key, true);
        agents_installed += 1;
    }
    if agents_installed == 0 {
        println!("  \x1b[2m(no agent instructions needed)\x1b[0m");
    }

    println!();
    println!("\x1b[1m4. Creating ~/.lean-ctx directory...\x1b[0m");
    let lean_dir = home.join(".lean-ctx");
    if !lean_dir.exists() {
        let _ = std::fs::create_dir_all(&lean_dir);
        println!("  \x1b[32m✓\x1b[0m Created {}", lean_dir.display());
    } else {
        println!("  \x1b[32m✓\x1b[0m Already exists");
    }

    println!();
    println!("\x1b[1m5. Running diagnostics...\x1b[0m");
    crate::doctor::run();

    println!();
    println!("\x1b[1;32m✓ Setup complete!\x1b[0m");

    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.contains("zsh") {
        println!("  Run: \x1b[1msource ~/.zshrc\x1b[0m to activate shell hooks");
    } else if shell.contains("fish") {
        println!("  Run: \x1b[1msource ~/.config/fish/config.fish\x1b[0m to activate shell hooks");
    } else if shell.contains("bash") {
        println!("  Run: \x1b[1msource ~/.bashrc\x1b[0m to activate shell hooks");
    } else {
        println!("  Restart your shell to activate hooks");
    }

    if !configured.is_empty() {
        println!(
            "  Restart your editor{} to load lean-ctx MCP tools",
            if configured.len() > 1 { "s" } else { "" }
        );
    }
}

fn build_targets(home: &std::path::Path, _binary: &str) -> Vec<EditorTarget> {
    vec![
        EditorTarget {
            name: "Cursor",
            agent_key: "cursor",
            config_path: home.join(".cursor/mcp.json"),
            detect_path: home.join(".cursor"),
            config_type: ConfigType::McpJson,
        },
        EditorTarget {
            name: "Claude Code",
            agent_key: "claude",
            config_path: home.join(".claude.json"),
            detect_path: detect_claude_path(),
            config_type: ConfigType::McpJson,
        },
        EditorTarget {
            name: "Windsurf",
            agent_key: "",
            config_path: home.join(".codeium/windsurf/mcp_config.json"),
            detect_path: home.join(".codeium/windsurf"),
            config_type: ConfigType::McpJson,
        },
        EditorTarget {
            name: "Codex CLI",
            agent_key: "codex",
            config_path: home.join(".codex/config.toml"),
            detect_path: home.join(".codex"),
            config_type: ConfigType::Codex,
        },
        EditorTarget {
            name: "Gemini CLI",
            agent_key: "",
            config_path: home.join(".gemini/settings/mcp.json"),
            detect_path: home.join(".gemini"),
            config_type: ConfigType::McpJson,
        },
        EditorTarget {
            name: "Zed",
            agent_key: "",
            config_path: zed_settings_path(home),
            detect_path: zed_config_dir(home),
            config_type: ConfigType::Zed,
        },
        EditorTarget {
            name: "VS Code / Copilot",
            agent_key: "",
            config_path: vscode_mcp_path(),
            detect_path: detect_vscode_path(),
            config_type: ConfigType::VsCodeMcp,
        },
    ]
}

fn detect_claude_path() -> PathBuf {
    if let Ok(output) = std::process::Command::new("which").arg("claude").output() {
        if output.status.success() {
            return PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
        }
    }
    if let Some(home) = dirs::home_dir() {
        let claude_json = home.join(".claude.json");
        if claude_json.exists() {
            return claude_json;
        }
    }
    PathBuf::from("/nonexistent")
}

fn zed_settings_path(home: &std::path::Path) -> PathBuf {
    home.join(".config/zed/settings.json")
}

fn zed_config_dir(home: &std::path::Path) -> PathBuf {
    home.join(".config/zed")
}

fn write_config(target: &EditorTarget, binary: &str) -> Result<(), String> {
    if let Some(parent) = target.config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    match target.config_type {
        ConfigType::McpJson => write_mcp_json(target, binary),
        ConfigType::Zed => write_zed_config(target, binary),
        ConfigType::Codex => write_codex_config(target, binary),
        ConfigType::VsCodeMcp => write_vscode_mcp(target, binary),
    }
}

fn write_mcp_json(target: &EditorTarget, binary: &str) -> Result<(), String> {
    if target.config_path.exists() {
        let content = std::fs::read_to_string(&target.config_path).map_err(|e| e.to_string())?;

        if content.contains("lean-ctx") {
            return Ok(());
        }

        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object_mut() {
                let servers = obj
                    .entry("mcpServers")
                    .or_insert_with(|| serde_json::json!({}));
                if let Some(servers_obj) = servers.as_object_mut() {
                    servers_obj.insert(
                        "lean-ctx".to_string(),
                        serde_json::json!({ "command": binary }),
                    );
                }
                let formatted = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
                std::fs::write(&target.config_path, formatted).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
    }

    let content = serde_json::to_string_pretty(&serde_json::json!({
        "mcpServers": {
            "lean-ctx": {
                "command": binary
            }
        }
    }))
    .map_err(|e| e.to_string())?;

    std::fs::write(&target.config_path, content).map_err(|e| e.to_string())
}

fn write_zed_config(target: &EditorTarget, binary: &str) -> Result<(), String> {
    if target.config_path.exists() {
        let content = std::fs::read_to_string(&target.config_path).map_err(|e| e.to_string())?;

        if content.contains("lean-ctx") {
            return Ok(());
        }

        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object_mut() {
                let servers = obj
                    .entry("context_servers")
                    .or_insert_with(|| serde_json::json!({}));
                if let Some(servers_obj) = servers.as_object_mut() {
                    servers_obj.insert(
                        "lean-ctx".to_string(),
                        serde_json::json!({
                            "source": "custom",
                            "command": binary,
                            "args": [],
                            "env": {}
                        }),
                    );
                }
                let formatted = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
                std::fs::write(&target.config_path, formatted).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
    }

    let content = serde_json::to_string_pretty(&serde_json::json!({
        "context_servers": {
            "lean-ctx": {
                "source": "custom",
                "command": binary,
                "args": [],
                "env": {}
            }
        }
    }))
    .map_err(|e| e.to_string())?;

    std::fs::write(&target.config_path, content).map_err(|e| e.to_string())
}

fn write_codex_config(target: &EditorTarget, binary: &str) -> Result<(), String> {
    if target.config_path.exists() {
        let content = std::fs::read_to_string(&target.config_path).map_err(|e| e.to_string())?;

        if content.contains("lean-ctx") {
            return Ok(());
        }

        let mut new_content = content.clone();
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str(&format!(
            "\n[mcp_servers.lean-ctx]\ncommand = \"{}\"\nargs = []\n",
            binary
        ));
        std::fs::write(&target.config_path, new_content).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let content = format!(
        "[mcp_servers.lean-ctx]\ncommand = \"{}\"\nargs = []\n",
        binary
    );
    std::fs::write(&target.config_path, content).map_err(|e| e.to_string())
}

fn write_vscode_mcp(target: &EditorTarget, binary: &str) -> Result<(), String> {
    if target.config_path.exists() {
        let content = std::fs::read_to_string(&target.config_path).map_err(|e| e.to_string())?;
        if content.contains("lean-ctx") {
            return Ok(());
        }
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object_mut() {
                let servers = obj
                    .entry("servers")
                    .or_insert_with(|| serde_json::json!({}));
                if let Some(servers_obj) = servers.as_object_mut() {
                    servers_obj.insert(
                        "lean-ctx".to_string(),
                        serde_json::json!({ "command": binary, "args": [] }),
                    );
                }
                let formatted = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
                std::fs::write(&target.config_path, formatted).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
    }

    if let Some(parent) = target.config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let content = serde_json::to_string_pretty(&serde_json::json!({
        "servers": {
            "lean-ctx": {
                "command": binary,
                "args": []
            }
        }
    }))
    .map_err(|e| e.to_string())?;

    std::fs::write(&target.config_path, content).map_err(|e| e.to_string())
}

fn detect_vscode_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let vscode = home.join("Library/Application Support/Code/User/settings.json");
            if vscode.exists() {
                return vscode;
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            let vscode = home.join(".config/Code/User/settings.json");
            if vscode.exists() {
                return vscode;
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let vscode = PathBuf::from(appdata).join("Code/User/settings.json");
            if vscode.exists() {
                return vscode;
            }
        }
    }
    if let Ok(output) = std::process::Command::new("which").arg("code").output() {
        if output.status.success() {
            return PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
        }
    }
    PathBuf::from("/nonexistent")
}

fn vscode_mcp_path() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        #[cfg(target_os = "macos")]
        {
            return home.join("Library/Application Support/Code/User/mcp.json");
        }
        #[cfg(target_os = "linux")]
        {
            return home.join(".config/Code/User/mcp.json");
        }
        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                return PathBuf::from(appdata).join("Code/User/mcp.json");
            }
        }
        #[allow(unreachable_code)]
        home.join(".config/Code/User/mcp.json")
    } else {
        PathBuf::from("/nonexistent")
    }
}
