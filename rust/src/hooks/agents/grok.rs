//! Grok (xAI Grok Build / `grok` CLI) integration.
//!
//! Config layout (documented under `~/.grok/docs/user-guide/`):
//! - MCP: `~/.grok/config.toml` → `[mcp_servers.lean-ctx]` (same shape as Codex)
//! - Hooks: `~/.grok/hooks/*.json` Claude-style nested PreToolUse
//! - Rules: `~/.grok/AGENTS.md` (global) + project AGENTS.md
//! - Skills: `~/.grok/skills/lean-ctx/SKILL.md`
//!
//! Grok PreToolUse blocking decisions use `{"decision":"allow|deny"}`. The shared
//! dual-format hook emitters also emit that field so rewrite/redirect/deny work.

use super::super::{
    HookMode, mcp_server_quiet_mode, resolve_binary_path, resolve_hook_command_binary,
    should_register_mcp, write_file,
};

pub(crate) fn install_grok_hook_with_mode(global: bool, mode: HookMode) {
    let Some(home) = crate::core::home::resolve_home_dir() else {
        tracing::error!("Cannot resolve home directory");
        return;
    };

    if should_register_mcp() {
        install_grok_mcp(&home);
    }
    install_grok_hook_config(&home, mode);

    // Global rules live in ~/.grok/AGENTS.md via rules_inject (setup_single_agent).
    // Project AGENTS.md pointer is shared with other agents.
    let scope = crate::core::config::Config::load().rules_scope_effective();
    if !global
        && matches!(
            scope,
            crate::core::config::RulesScope::Project | crate::core::config::RulesScope::Both
        )
    {
        super::super::install_project_rules_for_agents(&["grok"]);
    }

    if !mcp_server_quiet_mode() {
        let mode_name = match mode {
            HookMode::Mcp => "mcp",
            HookMode::Hybrid => "hybrid",
            HookMode::Replace => "replace",
        };
        eprintln!(
            "  \x1b[32m✓\x1b[0m Grok configured ({mode_name} mode, ~/.grok). Restart grok to activate."
        );
    }
}

fn install_grok_mcp(home: &std::path::Path) {
    let binary = resolve_binary_path();
    let config_path = home.join(".grok/config.toml");
    let target = crate::core::editor_registry::EditorTarget {
        name: "Grok",
        agent_key: "grok".to_string(),
        config_path: config_path.clone(),
        detect_path: home.join(".grok"),
        // Same TOML shape as Codex: [mcp_servers.lean-ctx] command/args.
        config_type: crate::core::editor_registry::ConfigType::Codex,
    };

    match crate::core::editor_registry::write_config_with_options(
        &target,
        &binary,
        crate::core::editor_registry::WriteOptions {
            overwrite_invalid: true,
        },
    ) {
        Ok(res) => {
            if mcp_server_quiet_mode() {
                return;
            }
            match res.action {
                crate::core::editor_registry::WriteAction::Created => {
                    eprintln!("  \x1b[32m✓\x1b[0m Grok MCP configured at ~/.grok/config.toml");
                }
                crate::core::editor_registry::WriteAction::Updated => {
                    eprintln!("  \x1b[32m✓\x1b[0m Grok MCP updated at ~/.grok/config.toml");
                }
                crate::core::editor_registry::WriteAction::Already => {
                    eprintln!("  Grok MCP already configured at ~/.grok/config.toml");
                }
            }
        }
        Err(e) => tracing::error!("Failed to configure Grok MCP: {e}"),
    }
}

fn install_grok_hook_config(home: &std::path::Path, mode: HookMode) {
    let hooks_dir = home.join(".grok/hooks");
    if let Err(e) = std::fs::create_dir_all(&hooks_dir) {
        tracing::error!("Failed to create {}: {e}", hooks_dir.display());
        return;
    }

    let binary = resolve_hook_command_binary();
    let rewrite_cmd = format!("{binary} hook rewrite");
    let redirect_cmd = format!("{binary} hook redirect");
    let deny_cmd = format!("{binary} hook deny");
    let observe_cmd = format!("{binary} hook observe");

    // Grok aliases Claude tool names → native ones in matchers (Bash→run_terminal_command,
    // Read→read_file, Grep→grep, Glob|ListDir→list_dir), so Claude-style matchers work.
    let bash_matcher = if cfg!(windows) {
        "Bash|bash|PowerShell|powershell|run_terminal_command"
    } else {
        "Bash|bash|run_terminal_command"
    };
    let redirect_matcher = "Read|Grep|Glob|read_file|grep|list_dir";

    let mut pretool: Vec<serde_json::Value> = vec![serde_json::json!({
        "matcher": bash_matcher,
        "hooks": [{
            "type": "command",
            "command": rewrite_cmd,
            "timeout": 15
        }]
    })];

    match mode {
        HookMode::Replace => {
            // Deny Grep/Glob; keep Read as redirect (compressed path) when MCP is up.
            pretool.push(serde_json::json!({
                "matcher": "Read|read_file",
                "hooks": [{
                    "type": "command",
                    "command": redirect_cmd,
                    "timeout": 15
                }]
            }));
            pretool.push(serde_json::json!({
                "matcher": "Grep|Glob|grep|list_dir",
                "hooks": [{
                    "type": "command",
                    "command": deny_cmd,
                    "timeout": 10
                }]
            }));
        }
        HookMode::Hybrid | HookMode::Mcp => {
            pretool.push(serde_json::json!({
                "matcher": redirect_matcher,
                "hooks": [{
                    "type": "command",
                    "command": redirect_cmd,
                    "timeout": 15
                }]
            }));
        }
    }

    let root = serde_json::json!({
        "hooks": {
            "PreToolUse": pretool,
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": observe_cmd.clone(),
                    "timeout": 5
                }]
            }],
            "PreCompact": [{
                "hooks": [{
                    "type": "command",
                    "command": observe_cmd,
                    "timeout": 5
                }]
            }]
        }
    });

    let path = hooks_dir.join("lean-ctx.json");
    let formatted = serde_json::to_string_pretty(&root).unwrap_or_default();
    write_file(&path, &formatted);

    if !mcp_server_quiet_mode() {
        eprintln!("  \x1b[32m✓\x1b[0m Grok hooks installed at ~/.grok/hooks/lean-ctx.json");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grok_hook_install_writes_mcp_and_hooks() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        std::fs::create_dir_all(home.join(".grok")).unwrap();
        std::fs::write(home.join(".grok/config.toml"), "[ui]\ntheme = \"test\"\n").unwrap();

        // Call helpers with explicit home — no process-wide HOME mutation.
        install_grok_mcp(home);
        install_grok_hook_config(home, HookMode::Hybrid);

        let config = std::fs::read_to_string(home.join(".grok/config.toml")).unwrap();
        assert!(
            config.contains("[mcp_servers.lean-ctx]"),
            "expected MCP section: {config}"
        );
        assert!(
            config.contains("command"),
            "expected command key in MCP section: {config}"
        );

        let hooks = std::fs::read_to_string(home.join(".grok/hooks/lean-ctx.json")).unwrap();
        assert!(hooks.contains("hook rewrite"), "hooks: {hooks}");
        assert!(hooks.contains("hook redirect"), "hooks: {hooks}");
        assert!(hooks.contains("PreToolUse"), "hooks: {hooks}");
    }

    #[test]
    fn grok_replace_mode_installs_deny_hook() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        std::fs::create_dir_all(home.join(".grok")).unwrap();

        install_grok_hook_config(home, HookMode::Replace);

        let hooks = std::fs::read_to_string(home.join(".grok/hooks/lean-ctx.json")).unwrap();
        assert!(hooks.contains("hook deny"), "replace hooks: {hooks}");
        assert!(hooks.contains("hook rewrite"), "replace hooks: {hooks}");
    }
}
