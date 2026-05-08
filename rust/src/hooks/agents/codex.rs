use super::super::{
    ensure_codex_hooks_enabled as shared_ensure_codex_hooks_enabled,
    install_codex_instruction_docs, mcp_server_quiet_mode, resolve_binary_path,
    upsert_lean_ctx_codex_hook_entries, write_file,
};

pub fn install_codex_hook() {
    let Some(home) = crate::core::home::resolve_home_dir() else {
        tracing::error!("Cannot resolve home directory");
        return;
    };

    let codex_dir = home.join(".codex");
    let _ = std::fs::create_dir_all(&codex_dir);

    let hook_config_changed = install_codex_hook_config(&home);
    let installed_docs = install_codex_instruction_docs(&codex_dir);

    if !mcp_server_quiet_mode() {
        if hook_config_changed {
            eprintln!(
                "Installed Codex-compatible SessionStart/PreToolUse hooks at {}",
                codex_dir.display()
            );
        }
        if installed_docs {
            eprintln!("Installed Codex instructions at {}", codex_dir.display());
        } else {
            eprintln!("Codex AGENTS.md already configured.");
        }
    }
}

fn install_codex_hook_config(home: &std::path::Path) -> bool {
    let binary = resolve_binary_path();
    let session_start_cmd = format!("{binary} hook codex-session-start");
    let pre_tool_use_cmd = format!("{binary} hook codex-pretooluse");
    let codex_dir = home.join(".codex");
    let hooks_json_path = codex_dir.join("hooks.json");

    let mut changed = false;
    let mut root = if hooks_json_path.exists() {
        if let Some(parsed) = std::fs::read_to_string(&hooks_json_path)
            .ok()
            .and_then(|content| crate::core::jsonc::parse_jsonc(&content).ok())
        {
            parsed
        } else {
            changed = true;
            serde_json::json!({ "hooks": {} })
        }
    } else {
        changed = true;
        serde_json::json!({ "hooks": {} })
    };

    if upsert_lean_ctx_codex_hook_entries(&mut root, &session_start_cmd, &pre_tool_use_cmd) {
        changed = true;
    }
    if changed {
        write_file(
            &hooks_json_path,
            &serde_json::to_string_pretty(&root).unwrap_or_default(),
        );
    }

    let rewrite_path = codex_dir.join("hooks").join("lean-ctx-rewrite-codex.sh");
    if rewrite_path.exists() && std::fs::remove_file(&rewrite_path).is_ok() {
        changed = true;
    }

    let config_toml_path = codex_dir.join("config.toml");
    let config_content = std::fs::read_to_string(&config_toml_path).unwrap_or_default();

    // Hybrid mode: ensure MCP server entry exists in config.toml so Codex
    // Desktop/Cloud can reach lean-ctx even without CLI hooks.
    let mcp_updated = ensure_codex_mcp_server(&config_content, &binary);
    let hooks_updated =
        ensure_codex_hooks_enabled(mcp_updated.as_deref().unwrap_or(&config_content));

    let final_content = hooks_updated
        .or(mcp_updated)
        .unwrap_or_else(|| config_content.clone());
    if final_content != config_content {
        write_file(&config_toml_path, &final_content);
        changed = true;
        if !mcp_server_quiet_mode() {
            eprintln!(
                "Updated Codex config (MCP server + hooks) in {}",
                config_toml_path.display()
            );
        }
    }

    changed
}

fn ensure_codex_mcp_server(config_content: &str, binary: &str) -> Option<String> {
    if config_content.contains("[mcp_servers.lean-ctx]") {
        return None;
    }
    let mut out = config_content.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!(
        "\n[mcp_servers.lean-ctx]\ncommand = \"{binary}\"\nargs = []\n"
    ));
    Some(out)
}

fn ensure_codex_hooks_enabled(config_content: &str) -> Option<String> {
    shared_ensure_codex_hooks_enabled(config_content)
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_codex_hooks_enabled, ensure_codex_mcp_server, upsert_lean_ctx_codex_hook_entries,
    };
    use serde_json::json;

    #[test]
    fn upsert_replaces_legacy_codex_rewrite_but_keeps_custom_hooks() {
        let mut input = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [{
                            "type": "command",
                            "command": "/opt/homebrew/bin/lean-ctx hook rewrite",
                            "timeout": 15
                        }]
                    },
                    {
                        "matcher": "Bash",
                        "hooks": [{
                            "type": "command",
                            "command": "echo keep-me",
                            "timeout": 5
                        }]
                    }
                ],
                "SessionStart": [
                    {
                        "matcher": "startup|resume|clear",
                        "hooks": [{
                            "type": "command",
                            "command": "lean-ctx hook codex-session-start",
                            "timeout": 15
                        }]
                    }
                ],
                "PostToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [{
                            "type": "command",
                            "command": "echo keep-post",
                            "timeout": 5
                        }]
                    }
                ]
            }
        });

        let changed = upsert_lean_ctx_codex_hook_entries(
            &mut input,
            "lean-ctx hook codex-session-start",
            "lean-ctx hook codex-pretooluse",
        );
        assert!(changed, "legacy hooks should be migrated");

        let pre_tool_use = input["hooks"]["PreToolUse"]
            .as_array()
            .expect("PreToolUse array should remain");
        assert_eq!(pre_tool_use.len(), 2, "custom hook should be preserved");
        assert_eq!(
            pre_tool_use[0]["hooks"][0]["command"].as_str(),
            Some("echo keep-me")
        );
        assert_eq!(
            pre_tool_use[1]["hooks"][0]["command"].as_str(),
            Some("lean-ctx hook codex-pretooluse")
        );
        assert_eq!(
            input["hooks"]["SessionStart"][0]["hooks"][0]["command"].as_str(),
            Some("lean-ctx hook codex-session-start")
        );
        assert_eq!(
            input["hooks"]["PostToolUse"][0]["hooks"][0]["command"].as_str(),
            Some("echo keep-post")
        );
    }

    #[test]
    fn ignores_non_lean_ctx_codex_entries() {
        let custom = json!({
            "matcher": "Bash",
            "hooks": [{
                "type": "command",
                "command": "echo keep-me",
                "timeout": 5
            }]
        });
        assert!(
            !crate::hooks::support::is_lean_ctx_codex_managed_entry("PreToolUse", &custom),
            "custom Codex hooks must be preserved"
        );
    }

    #[test]
    fn detects_managed_codex_session_start_entry() {
        let managed = json!({
            "matcher": "startup|resume|clear",
            "hooks": [{
                "type": "command",
                "command": "/opt/homebrew/bin/lean-ctx hook codex-session-start",
                "timeout": 15
            }]
        });
        assert!(crate::hooks::support::is_lean_ctx_codex_managed_entry(
            "SessionStart",
            &managed
        ));
    }

    #[test]
    fn ensure_codex_hooks_enabled_updates_existing_features_flag() {
        let input = "\
[features]
other = true
codex_hooks = false

[mcp_servers.other]
command = \"other\"
";

        let output =
            ensure_codex_hooks_enabled(input).expect("codex_hooks=false should be migrated");

        assert!(output.contains("[features]\nother = true\ncodex_hooks = true\n"));
        assert!(!output.contains("codex_hooks = false"));
    }

    #[test]
    fn ensure_codex_hooks_enabled_moves_stray_assignment_into_features_section() {
        let input = "\
[features]
other = true

[mcp_servers.lean-ctx]
command = \"lean-ctx\"
codex_hooks = true
";

        let output = ensure_codex_hooks_enabled(input)
            .expect("stray codex_hooks assignment should be normalized");

        assert!(output.contains("[features]\nother = true\ncodex_hooks = true\n"));
        assert_eq!(output.matches("codex_hooks = true").count(), 1);
        assert!(
            !output.contains("[mcp_servers.lean-ctx]\ncommand = \"lean-ctx\"\ncodex_hooks = true")
        );
    }

    #[test]
    fn ensure_codex_hooks_enabled_adds_features_section_when_missing() {
        let input = "\
[mcp_servers.lean-ctx]
command = \"lean-ctx\"
";

        let output =
            ensure_codex_hooks_enabled(input).expect("missing features section should be added");

        assert!(output.ends_with("\n[features]\ncodex_hooks = true\n"));
    }

    #[test]
    fn install_codex_docs_preserves_existing_user_instructions() {
        let tmp = std::env::temp_dir().join("lean-ctx-test-codex-preserve");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let agents_md = tmp.join("AGENTS.md");
        let user_content = "# My Custom Instructions\n\nDo not change my codebase style.\n\n## Rules\n- Always use tabs\n- No semicolons\n";
        std::fs::write(&agents_md, user_content).unwrap();

        crate::hooks::support::install_codex_instruction_docs(&tmp);

        let result = std::fs::read_to_string(&agents_md).unwrap();
        assert!(
            result.contains("My Custom Instructions"),
            "user content must be preserved"
        );
        assert!(
            result.contains("Always use tabs"),
            "user rules must be preserved"
        );
        assert!(
            result.contains("<!-- lean-ctx -->"),
            "lean-ctx block must be appended"
        );
        assert!(
            result.contains("LEAN-CTX.md (same directory)"),
            "lean-ctx reference must be present"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn install_codex_docs_updates_only_marked_block() {
        let tmp = std::env::temp_dir().join("lean-ctx-test-codex-marked");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let agents_md = tmp.join("AGENTS.md");
        let content_with_block = "# My Instructions\n\nCustom rule here.\n\n<!-- lean-ctx -->\n## lean-ctx\n\n@OLD-LEAN-CTX.md\n<!-- /lean-ctx -->\n\n## Other Section\nKeep this.\n";
        std::fs::write(&agents_md, content_with_block).unwrap();

        crate::hooks::support::install_codex_instruction_docs(&tmp);

        let result = std::fs::read_to_string(&agents_md).unwrap();
        assert!(
            result.contains("Custom rule here."),
            "user content before block preserved"
        );
        assert!(
            result.contains("Other Section"),
            "user content after block preserved"
        );
        assert!(
            result.contains("LEAN-CTX.md (same directory)"),
            "block updated to current reference"
        );
        assert!(
            !result.contains("OLD-LEAN-CTX"),
            "old block content replaced"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn ensure_mcp_server_adds_section_when_missing() {
        let input = "[features]\ncodex_hooks = true\n";
        let result = ensure_codex_mcp_server(input, "lean-ctx").expect("should add MCP section");
        assert!(result.contains("[mcp_servers.lean-ctx]"));
        assert!(result.contains("command = \"lean-ctx\""));
        assert!(result.contains("args = []"));
        assert!(result.contains("[features]\ncodex_hooks = true\n"));
    }

    #[test]
    fn ensure_mcp_server_noop_when_present() {
        let input = "[mcp_servers.lean-ctx]\ncommand = \"lean-ctx\"\nargs = []\n";
        assert!(
            ensure_codex_mcp_server(input, "lean-ctx").is_none(),
            "should not modify config when MCP section already exists"
        );
    }

    #[test]
    fn ensure_mcp_server_preserves_existing_sections() {
        let input = "[mcp_servers.other]\ncommand = \"other\"\n";
        let result = ensure_codex_mcp_server(input, "/usr/bin/lean-ctx")
            .expect("should add lean-ctx section");
        assert!(result.contains("[mcp_servers.other]"));
        assert!(result.contains("[mcp_servers.lean-ctx]"));
        assert!(result.contains("command = \"/usr/bin/lean-ctx\""));
    }
}
