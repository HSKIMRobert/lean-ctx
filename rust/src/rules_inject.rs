use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const MARKER: &str = "# lean-ctx — Context Engineering Layer";
const END_MARKER: &str = "<!-- /lean-ctx -->";
const RULES_VERSION: &str = "lean-ctx-rules-v11";

pub const RULES_MARKER: &str = MARKER;
pub const RULES_END_MARKER: &str = END_MARKER;
pub const RULES_VERSION_STR: &str = RULES_VERSION;

pub fn rules_dedicated_markdown() -> &'static str {
    RULES_DEDICATED
}

pub fn rules_shared_content() -> &'static str {
    RULES_SHARED
}

/// Compact, agent-agnostic tool-mapping summary injected as `SessionStart`
/// `additionalContext` in `rules_injection = "dedicated"` mode.
///
/// It is the always-loaded counterpart of the (now-skipped) CLAUDE.md/AGENTS.md
/// marker block, so it carries the same essential per-session footprint. The
/// full ruleset stays lazy in the removable dedicated rules file. No HTML markers
/// and no `@import` pointer (Codex has no `@import`), so it renders cleanly as a
/// developer/system reminder on both Claude and Codex.
pub fn dedicated_session_summary() -> &'static str {
    DEDICATED_SESSION_SUMMARY
}

const DEDICATED_SESSION_SUMMARY: &str =
    "lean-ctx is active — prefer its tools over native equivalents to save context tokens:
- ctx_read  → instead of Read/cat/head/tail (cached, 10 modes, re-reads ~13 tokens)
- ctx_shell → instead of bash/Shell (95+ output-compression patterns)
- ctx_search → instead of Grep/rg/find (compact, token-efficient results)
- ctx_tree  → instead of ls/find (compact directory maps)
Native Edit/Write/Glob stay as-is. If Edit needs Read and Read is unavailable, use ctx_edit.
NEVER use native Read/Grep/Shell when a ctx_* equivalent exists.";

/// Dedicated-mode rules file for OpenCode, auto-loaded via `opencode.json`
/// `instructions[]` (absolute path) instead of polluting `~/.config/opencode/AGENTS.md`.
pub fn opencode_dedicated_rules_path(home: &std::path::Path) -> PathBuf {
    home.join(".config/opencode/rules/lean-ctx.md")
}

/// Dedicated-mode rules file for Gemini CLI, auto-loaded via `settings.json`
/// `context.fileName` instead of polluting `~/.gemini/GEMINI.md`. The filename is
/// deliberately distinctive: Gemini discovers context files *by name* across the
/// project tree, so a generic `lean-ctx.md` would get picked up everywhere.
pub fn gemini_dedicated_rules_path(home: &std::path::Path) -> PathBuf {
    home.join(".gemini").join(GEMINI_DEDICATED_CONTEXT_FILENAME)
}

/// The `context.fileName` entry registered for Gemini in dedicated mode.
pub const GEMINI_DEDICATED_CONTEXT_FILENAME: &str = "LEANCTX.md";

// ---------------------------------------------------------------------------
// Rules content for SHARED config files (appended to user's existing config).
// LITM-optimized: critical instruction at START and END of block.
// Used for: CLAUDE.md, instructions.md, GEMINI.md, copilot-instructions.md
// ---------------------------------------------------------------------------
const RULES_SHARED: &str = r#"# lean-ctx — Context Engineering Layer
<!-- lean-ctx-rules-v11 -->

## Tool Mapping (MANDATORY — use instead of native equivalents)
| Instead of | Use | Example |
|------------|-----|---------|
| Read/cat/head/tail | `ctx_read(path, mode)` | `ctx_read("src/main.rs", "full")` |
| Grep/rg/find | `ctx_search(pattern, path)` | `ctx_search("fn handle", "src/")` |
| Shell/bash | `ctx_shell(command)` | `ctx_shell("cargo test")` |
| Edit (when Read unavailable) | `ctx_edit(path, old, new)` | `ctx_edit("f.rs", "old", "new")` |

## ctx_read Mode Selection
| Goal | Mode | When |
|------|------|------|
| Edit this file | `full` | Before any edit |
| Understand API | `signatures` | Context-only, won't edit |
| Re-read after edit | `diff` | Post-edit verification |
| Large file overview | `map` | >500 lines, won't edit |
| Specific region | `lines:N-M` | Know exact location |

## Workflow (follow this order)
1. **Orient:** `ctx_overview(task)` or `ctx_compose(task, path)` for unfamiliar tasks
2. **Locate:** `ctx_search(pattern, path)` for exact text; `ctx_semantic_search(query)` for concepts
3. **Read:** `ctx_read(path, mode)` with appropriate mode from table above
4. **Edit:** `ctx_edit(path, old_string, new_string)` or native Edit if available
5. **Verify:** `ctx_read(path, "diff")` + `ctx_shell("test command")`
6. **Record:** `ctx_knowledge(action="remember", content="...")` for non-obvious findings

## Session
- **Start:** `ctx_session(action="status")` + `ctx_knowledge(action="wakeup")`
- **End:** `ctx_session(action="decision", content="what was done + next steps")`
- **On [CHECKPOINT]:** `ctx_session(action="task", value="current status")`

NEVER use native Read/Grep/Shell when ctx_* equivalents are available.
<!-- /lean-ctx -->"#;

// ---------------------------------------------------------------------------
// Rules content for DEDICATED lean-ctx rule files (we control entire file).
// LITM-optimized with critical mapping at start and end.
// Used for: Windsurf, Zed, Cline, Roo Code, OpenCode, Continue, Aider
// ---------------------------------------------------------------------------
const RULES_DEDICATED: &str = r#"# lean-ctx — Context Engineering Layer
<!-- lean-ctx-rules-v11 -->

## Tool Mapping (MANDATORY — use instead of native equivalents)
| Instead of | Use | Example |
|------------|-----|---------|
| Read/cat/head/tail | `ctx_read(path, mode)` | `ctx_read("src/main.rs", "full")` |
| Grep/rg/find | `ctx_search(pattern, path)` | `ctx_search("fn handle", "src/")` |
| Shell/bash | `ctx_shell(command)` | `ctx_shell("cargo test")` |
| Edit (when Read unavailable) | `ctx_edit(path, old, new)` | `ctx_edit("f.rs", "old", "new")` |

## ctx_read Mode Selection
| Goal | Mode | When |
|------|------|------|
| Edit this file | `full` | Before any edit |
| Understand API | `signatures` | Context-only, won't edit |
| Re-read after edit | `diff` | Post-edit verification |
| Large file overview | `map` | >500 lines, won't edit |
| Specific region | `lines:N-M` | Know exact location |
| Unsure | `auto` | System selects optimal mode |

## Workflow (follow this order)
1. **Orient:** `ctx_overview(task)` or `ctx_compose(task, path)` for unfamiliar tasks
2. **Locate:** `ctx_search(pattern, path)` for exact text; `ctx_semantic_search(query)` for concepts
3. **Read:** `ctx_read(path, mode)` with appropriate mode from table above
4. **Edit:** `ctx_edit(path, old_string, new_string)` or native Edit if available
5. **Verify:** `ctx_read(path, "diff")` + `ctx_shell("test command")`
6. **Record:** `ctx_knowledge(action="remember", content="...")` for non-obvious findings

## Proactive (use without being asked)
- `ctx_overview(task)` — at session start for orientation
- `ctx_compress` — when context grows large (at phase boundaries)
- `ctx_knowledge(action="wakeup")` — at session start to surface prior findings

## Compression Bypass (only when compressed output hides needed detail)
`ctx_read(path, "lines:N-M")` → `ctx_read(path, "full")` → `ctx_shell(cmd, raw=true)`
Return to compressed defaults after one expanded retrieval.

## Risk Gate (before high-impact edits)
Before editing exported symbols, auth, DB schemas, or 3+ files: run `ctx_impact(action="analyze")`
and `ctx_callgraph(action="callers")` to confirm blast radius.

## Session
- **Start:** `ctx_session(action="status")` + `ctx_knowledge(action="wakeup")`
- **End:** `ctx_session(action="decision", content="what was done + next steps")`
- **On [CHECKPOINT]:** `ctx_session(action="task", value="current status")`

NEVER use native Read/Grep/Shell when ctx_* equivalents are available.
<!-- /lean-ctx -->"#;

// ---------------------------------------------------------------------------
// Rules for Cursor MDC format (dedicated file with frontmatter).
// ---------------------------------------------------------------------------
const RULES_CURSOR_MDC: &str = include_str!("templates/lean-ctx.mdc");

// ---------------------------------------------------------------------------

struct RulesTarget {
    name: &'static str,
    path: PathBuf,
    format: RulesFormat,
}

enum RulesFormat {
    SharedMarkdown,
    DedicatedMarkdown,
    CursorMdc,
}

#[derive(Debug, Default)]
pub struct InjectResult {
    pub injected: Vec<String>,
    pub updated: Vec<String>,
    pub already: Vec<String>,
    pub errors: Vec<String>,
    pub backed_up: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesTargetStatus {
    pub name: String,
    pub detected: bool,
    pub path: String,
    pub state: String,
    pub note: Option<String>,
}

pub fn inject_all_rules(home: &std::path::Path) -> InjectResult {
    let cfg = crate::core::config::Config::load();
    if cfg.rules_scope_effective() == crate::core::config::RulesScope::Project {
        return InjectResult::default();
    }

    let targets = build_rules_targets(home, cfg.rules_injection_effective());

    let mut result = InjectResult::default();

    for target in &targets {
        if !is_tool_detected(target, home) {
            continue;
        }

        let bak_path = target.path.with_extension(format!(
            "{}.bak",
            target
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
        ));
        let bak_existed_before = bak_path.exists();
        let bak_mtime_before = bak_existed_before
            .then(|| {
                std::fs::metadata(&bak_path)
                    .ok()
                    .and_then(|m| m.modified().ok())
            })
            .flatten();

        match inject_rules(target) {
            Ok(RulesResult::Injected) => result.injected.push(target.name.to_string()),
            Ok(RulesResult::Updated) => {
                result.updated.push(target.name.to_string());
                let bak_is_new = if bak_existed_before {
                    std::fs::metadata(&bak_path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        != bak_mtime_before
                } else {
                    bak_path.exists()
                };
                if bak_is_new {
                    result
                        .backed_up
                        .push(bak_path.to_string_lossy().to_string());
                }
            }
            Ok(RulesResult::AlreadyPresent) => result.already.push(target.name.to_string()),
            Err(e) => result.errors.push(format!("{}: {e}", target.name)),
        }
    }

    result
}

/// Inject global rules for a single agent (by CLI key like "opencode", "cursor", etc.).
/// Used by `init --agent` to ensure global rules are written alongside MCP config.
pub fn inject_rules_for_agent(home: &std::path::Path, agent_key: &str) -> InjectResult {
    let cfg = crate::core::config::Config::load();
    if cfg.rules_scope_effective() == crate::core::config::RulesScope::Project {
        return InjectResult::default();
    }

    let targets = build_rules_targets(home, cfg.rules_injection_effective());
    let mut result = InjectResult::default();

    for target in &targets {
        if !match_agent_name(agent_key, target.name) {
            continue;
        }

        let bak_path = target.path.with_extension(format!(
            "{}.bak",
            target
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
        ));
        let bak_existed_before = bak_path.exists();

        match inject_rules(target) {
            Ok(RulesResult::Injected) => result.injected.push(target.name.to_string()),
            Ok(RulesResult::Updated) => {
                result.updated.push(target.name.to_string());
                if !bak_existed_before && bak_path.exists() {
                    result
                        .backed_up
                        .push(bak_path.to_string_lossy().to_string());
                }
            }
            Ok(RulesResult::AlreadyPresent) => result.already.push(target.name.to_string()),
            Err(e) => result.errors.push(format!("{}: {e}", target.name)),
        }
    }

    result
}

fn match_agent_name(cli_key: &str, target_name: &str) -> bool {
    let needle = cli_key.to_lowercase();
    let tn = target_name.to_lowercase();
    needle.contains(&tn)
        || tn.contains(&needle)
        || (needle.contains("cursor") && tn.contains("cursor"))
        || (needle.contains("claude") && tn.contains("claude"))
        || (needle.contains("windsurf") && tn.contains("windsurf"))
        || (needle.contains("codex") && tn.contains("claude"))
        || (needle.contains("zed") && tn.contains("zed"))
        || (needle.contains("copilot") && tn.contains("copilot"))
        || (needle.contains("jetbrains") && tn.contains("jetbrains"))
        || (needle.contains("kiro") && tn.contains("kiro"))
        || (needle.contains("gemini") && tn.contains("gemini"))
        || (needle == "opencode" && tn.contains("opencode"))
        || (needle == "cline" && tn.contains("cline"))
        || (needle == "roo" && tn.contains("roo"))
        || (needle == "amp" && tn.contains("amp"))
        || (needle == "trae" && tn.contains("trae"))
        || (needle == "amazonq" && tn.contains("amazon"))
        || (needle == "pi" && tn.contains("pi coding"))
        || (needle == "crush" && tn.contains("crush"))
        || (needle == "verdent" && tn.contains("verdent"))
        || (needle == "continue" && tn.contains("continue"))
        || (needle == "qwen" && tn.contains("qwen"))
        || (needle == "antigravity" && tn.contains("antigravity"))
        || (needle == "augment" && tn.contains("augment"))
        || (needle == "openclaw" && tn.contains("openclaw"))
        || (needle == "vscode" && (tn.contains("vs code") || tn.contains("vscode")))
}

/// Check if the rules file for a given MCP client is up-to-date.
/// Returns `Some(message)` if rules are stale/missing, `None` if current.
pub fn check_rules_freshness(client_name: &str) -> Option<String> {
    let home = dirs::home_dir()?;
    let injection = crate::core::config::Config::load().rules_injection_effective();
    let targets = build_rules_targets(&home, injection);

    let matched: Vec<&RulesTarget> = targets
        .iter()
        .filter(|t| match_agent_name(client_name, t.name))
        .collect();

    if matched.is_empty() {
        return None;
    }

    for target in &matched {
        if !target.path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&target.path).ok()?;
        if content.contains(MARKER) && !content.contains(RULES_VERSION) {
            return Some(format!(
                "[RULES OUTDATED] Your {} rules were written by an older lean-ctx version. \
                 Re-read your rules file ({}) or run `lean-ctx setup` to update, \
                 then start a new session for full compatibility.",
                target.name,
                target.path.display()
            ));
        }
    }

    None
}

pub fn collect_rules_status(home: &std::path::Path) -> Vec<RulesTargetStatus> {
    let injection = crate::core::config::Config::load().rules_injection_effective();
    let targets = build_rules_targets(home, injection);
    let mut out = Vec::new();

    for target in &targets {
        let detected = is_tool_detected(target, home);
        let path = target.path.to_string_lossy().to_string();

        let state = if !detected {
            "not_detected".to_string()
        } else if !target.path.exists() {
            "missing".to_string()
        } else {
            match std::fs::read_to_string(&target.path) {
                Ok(content) => {
                    if content.contains(MARKER) {
                        if content.contains(RULES_VERSION) {
                            "up_to_date".to_string()
                        } else {
                            "outdated".to_string()
                        }
                    } else {
                        "present_without_marker".to_string()
                    }
                }
                Err(_) => "read_error".to_string(),
            }
        };

        out.push(RulesTargetStatus {
            name: target.name.to_string(),
            detected,
            path,
            state,
            note: None,
        });
    }

    out
}

// ---------------------------------------------------------------------------
// Injection logic
// ---------------------------------------------------------------------------

enum RulesResult {
    Injected,
    Updated,
    AlreadyPresent,
}

fn rules_content(format: &RulesFormat) -> &'static str {
    match format {
        RulesFormat::SharedMarkdown => RULES_SHARED,
        RulesFormat::DedicatedMarkdown => RULES_DEDICATED,
        RulesFormat::CursorMdc => RULES_CURSOR_MDC,
    }
}

fn inject_rules(target: &RulesTarget) -> Result<RulesResult, String> {
    if target.path.exists() {
        let content = std::fs::read_to_string(&target.path).map_err(|e| e.to_string())?;
        if content.contains(MARKER) {
            if content.contains(RULES_VERSION) {
                return Ok(RulesResult::AlreadyPresent);
            }
            ensure_parent(&target.path)?;
            return match target.format {
                RulesFormat::SharedMarkdown => replace_markdown_section(&target.path, &content),
                RulesFormat::DedicatedMarkdown | RulesFormat::CursorMdc => {
                    write_dedicated(&target.path, rules_content(&target.format))
                }
            };
        }
    }

    ensure_parent(&target.path)?;

    match target.format {
        RulesFormat::SharedMarkdown => append_to_shared(&target.path),
        RulesFormat::DedicatedMarkdown | RulesFormat::CursorMdc => {
            write_dedicated(&target.path, rules_content(&target.format))
        }
    }
}

fn ensure_parent(path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn append_to_shared(path: &std::path::Path) -> Result<RulesResult, String> {
    let mut content = if path.exists() {
        std::fs::read_to_string(path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    if !content.is_empty() {
        content.push('\n');
    }
    content.push_str(RULES_SHARED);
    content.push('\n');

    crate::config_io::write_atomic_with_backup(path, &content)?;
    Ok(RulesResult::Injected)
}

fn replace_markdown_section(path: &std::path::Path, content: &str) -> Result<RulesResult, String> {
    let start = content.find(MARKER);
    let end = content.find(END_MARKER);

    let new_content = match (start, end) {
        (Some(s), Some(e)) => {
            let before = &content[..s];
            let after_end = e + END_MARKER.len();
            let after = content[after_end..].trim_start_matches('\n');
            let mut result = before.to_string();
            result.push_str(RULES_SHARED);
            if !after.is_empty() {
                result.push('\n');
                result.push_str(after);
            }
            result
        }
        (Some(s), None) => {
            let before = &content[..s];
            let mut result = before.to_string();
            result.push_str(RULES_SHARED);
            result.push('\n');
            result
        }
        _ => return Ok(RulesResult::AlreadyPresent),
    };

    crate::config_io::write_atomic_with_backup(path, &new_content)?;
    Ok(RulesResult::Updated)
}

fn write_dedicated(path: &std::path::Path, content: &'static str) -> Result<RulesResult, String> {
    if !path.exists() {
        crate::config_io::write_atomic_with_backup(path, content)?;
        return Ok(RulesResult::Injected);
    }

    let existing = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    if !existing.contains(MARKER) {
        crate::config_io::write_atomic_with_backup(path, content)?;
        return Ok(RulesResult::Injected);
    }

    let start = existing.find(MARKER);
    let end = existing.find(END_MARKER);

    let (before, after) = match (start, end) {
        (Some(s), Some(e)) => {
            let before = &existing[..s];
            let after_end = e + END_MARKER.len();
            let after = existing[after_end..].trim_start_matches('\n');
            (before.to_string(), after.to_string())
        }
        (Some(s), None) => (existing[..s].to_string(), String::new()),
        _ => (String::new(), String::new()),
    };

    let has_user_content = !before.trim().is_empty() || !after.trim().is_empty();

    if has_user_content {
        let new_section = if let Some(marker_pos) = content.find(MARKER) {
            &content[marker_pos..]
        } else {
            content
        };

        let mut result = before.clone();
        result.push_str(new_section);
        if !after.is_empty() {
            if !result.ends_with('\n') {
                result.push('\n');
            }
            result.push_str(&after);
        }
        if !result.ends_with('\n') {
            result.push('\n');
        }
        crate::config_io::write_atomic_with_backup(path, &result)?;
    } else {
        crate::config_io::write_atomic_with_backup(path, content)?;
    }

    Ok(RulesResult::Updated)
}

// ---------------------------------------------------------------------------
// Tool detection
// ---------------------------------------------------------------------------

fn is_tool_detected(target: &RulesTarget, home: &std::path::Path) -> bool {
    match target.name {
        "Claude Code" => {
            if command_exists("claude") {
                return true;
            }
            let state_dir = crate::core::editor_registry::claude_state_dir(home);
            crate::core::editor_registry::claude_mcp_json_path(home).exists() || state_dir.exists()
        }
        "Codex CLI" => {
            let codex_dir =
                crate::core::home::resolve_codex_dir().unwrap_or_else(|| home.join(".codex"));
            codex_dir.exists() || command_exists("codex")
        }
        "Cursor" => home.join(".cursor").exists(),
        "Windsurf" => home.join(".codeium/windsurf").exists(),
        "Gemini CLI" => home.join(".gemini").exists(),
        "VS Code" => detect_vscode_installed(home),
        "Copilot CLI" => home.join(".copilot").exists() || command_exists("copilot"),
        "Zed" => crate::core::editor_registry::zed_config_dir(home).exists(),
        "Cline" => detect_extension_installed(home, "saoudrizwan.claude-dev"),
        "Roo Code" => detect_extension_installed(home, "rooveterinaryinc.roo-cline"),
        "OpenCode" => home.join(".config/opencode").exists(),
        "Continue" => detect_extension_installed(home, "continue.continue"),
        "Amp" => command_exists("amp") || home.join(".ampcoder").exists(),
        "Qwen Code" => home.join(".qwen").exists(),
        "Trae" => home.join(".trae").exists(),
        "Amazon Q Developer" => home.join(".aws/amazonq").exists(),
        "JetBrains IDEs" => detect_jetbrains_installed(home),
        "Antigravity" => home.join(".gemini/antigravity").exists(),
        "Pi Coding Agent" => home.join(".pi").exists() || command_exists("pi"),
        "AWS Kiro" => home.join(".kiro").exists(),
        "Crush" => home.join(".config/crush").exists() || command_exists("crush"),
        "Verdent" => home.join(".verdent").exists(),
        // Augment ships as either the `auggie` CLI (writes to ~/.augment/) or
        // the VS Code extension (`augment.vscode-augment` globalStorage).
        "Augment" => {
            command_exists("auggie")
                || home.join(".augment").exists()
                || detect_extension_installed(home, "augment.vscode-augment")
        }
        "OpenClaw" => home.join(".openclaw").exists() || command_exists("openclaw"),
        "Hermes Agent" => home.join(".hermes").exists() || command_exists("hermes"),
        _ => false,
    }
}

fn command_exists(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("where")
        .arg(name)
        .output()
        .is_ok_and(|o| o.status.success());

    #[cfg(not(target_os = "windows"))]
    let result = std::process::Command::new("which")
        .arg(name)
        .output()
        .is_ok_and(|o| o.status.success());

    result
}

fn detect_vscode_installed(_home: &std::path::Path) -> bool {
    let check_dir = |dir: PathBuf| -> bool {
        dir.join("settings.json").exists() || dir.join("mcp.json").exists()
    };

    #[cfg(target_os = "macos")]
    if check_dir(_home.join("Library/Application Support/Code/User")) {
        return true;
    }
    #[cfg(target_os = "linux")]
    if check_dir(_home.join(".config/Code/User")) {
        return true;
    }
    #[cfg(target_os = "windows")]
    if let Ok(appdata) = std::env::var("APPDATA") {
        if check_dir(PathBuf::from(&appdata).join("Code/User")) {
            return true;
        }
    }
    false
}

fn detect_jetbrains_installed(home: &std::path::Path) -> bool {
    #[cfg(target_os = "macos")]
    if home.join("Library/Application Support/JetBrains").exists() {
        return true;
    }
    #[cfg(target_os = "linux")]
    if home.join(".config/JetBrains").exists() {
        return true;
    }
    home.join(".jb-mcp.json").exists()
}

fn detect_extension_installed(_home: &std::path::Path, extension_id: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        if _home
            .join(format!(
                "Library/Application Support/Code/User/globalStorage/{extension_id}"
            ))
            .exists()
        {
            return true;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if _home
            .join(format!(".config/Code/User/globalStorage/{extension_id}"))
            .exists()
        {
            return true;
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if std::path::PathBuf::from(&appdata)
                .join(format!("Code/User/globalStorage/{extension_id}"))
                .exists()
            {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Target definitions
// ---------------------------------------------------------------------------

fn build_rules_targets(
    home: &std::path::Path,
    injection: crate::core::config::RulesInjection,
) -> Vec<RulesTarget> {
    use crate::core::config::RulesInjection;

    // In dedicated mode the two AGENTS.md/GEMINI.md consumers write to a
    // lean-ctx-owned file instead of the user's shared instruction file;
    // discovery is wired up separately via opencode.json instructions[] /
    // .gemini/settings.json context.fileName (#343).
    let (gemini_path, gemini_format) = match injection {
        RulesInjection::Dedicated => (
            gemini_dedicated_rules_path(home),
            RulesFormat::DedicatedMarkdown,
        ),
        RulesInjection::Shared => (home.join(".gemini/GEMINI.md"), RulesFormat::SharedMarkdown),
    };
    let (opencode_path, opencode_format) = match injection {
        RulesInjection::Dedicated => (
            opencode_dedicated_rules_path(home),
            RulesFormat::DedicatedMarkdown,
        ),
        RulesInjection::Shared => (
            home.join(".config/opencode/AGENTS.md"),
            RulesFormat::SharedMarkdown,
        ),
    };

    vec![
        // --- Shared config files (append-only) ---
        RulesTarget {
            name: "Claude Code",
            path: crate::core::editor_registry::claude_rules_dir(home).join("lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Gemini CLI",
            path: gemini_path,
            format: gemini_format,
        },
        RulesTarget {
            name: "VS Code",
            path: copilot_instructions_path(home),
            format: RulesFormat::SharedMarkdown,
        },
        RulesTarget {
            name: "Copilot CLI",
            path: home.join(".copilot/instructions.md"),
            format: RulesFormat::SharedMarkdown,
        },
        // --- Dedicated lean-ctx rule files ---
        RulesTarget {
            name: "Cursor",
            path: home.join(".cursor/rules/lean-ctx.mdc"),
            format: RulesFormat::CursorMdc,
        },
        RulesTarget {
            name: "Windsurf",
            path: home.join(".codeium/windsurf/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Zed",
            // OS-aware: Zed's config dir is platform-specific (macOS uses
            // Application Support); keep rules co-located with the MCP config.
            path: crate::core::editor_registry::zed_config_dir(home).join("rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Cline",
            path: home.join(".cline/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Roo Code",
            path: home.join(".roo/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "OpenCode",
            path: opencode_path,
            format: opencode_format,
        },
        RulesTarget {
            name: "Continue",
            path: home.join(".continue/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Amp",
            path: home.join(".ampcoder/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Qwen Code",
            path: home.join(".qwen/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Trae",
            path: home.join(".trae/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Amazon Q Developer",
            path: home.join(".aws/amazonq/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "JetBrains IDEs",
            path: home.join(".jb-rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Antigravity",
            path: home.join(".gemini/antigravity/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Pi Coding Agent",
            path: home.join(".pi/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "AWS Kiro",
            path: home.join(".kiro/steering/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Verdent",
            path: home.join(".verdent/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Crush",
            path: home.join(".config/crush/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Augment",
            path: home.join(".augment/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "OpenClaw",
            path: home.join(".openclaw/rules/lean-ctx.md"),
            format: RulesFormat::DedicatedMarkdown,
        },
        RulesTarget {
            name: "Codex CLI",
            path: crate::core::home::resolve_codex_dir()
                .unwrap_or_else(|| home.join(".codex"))
                .join("instructions.md"),
            format: RulesFormat::SharedMarkdown,
        },
        RulesTarget {
            name: "Hermes Agent",
            path: home.join(".hermes/HERMES.md"),
            format: RulesFormat::SharedMarkdown,
        },
    ]
}

fn copilot_instructions_path(home: &std::path::Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        return home.join("Library/Application Support/Code/User/github-copilot-instructions.md");
    }
    #[cfg(target_os = "linux")]
    {
        return home.join(".config/Code/User/github-copilot-instructions.md");
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("Code/User/github-copilot-instructions.md");
        }
    }
    #[allow(unreachable_code)]
    home.join(".config/Code/User/github-copilot-instructions.md")
}

// ---------------------------------------------------------------------------
// SKILL.md installation
// ---------------------------------------------------------------------------

const SKILL_TEMPLATE: &str = include_str!("templates/SKILL.md");

struct SkillTarget {
    agent_key: &'static str,
    display_name: &'static str,
    skill_dir: PathBuf,
}

fn build_skill_targets(home: &std::path::Path) -> Vec<SkillTarget> {
    vec![
        SkillTarget {
            agent_key: "claude",
            display_name: "Claude Code",
            skill_dir: crate::setup::claude_config_dir(home).join("skills/lean-ctx"),
        },
        SkillTarget {
            agent_key: "cursor",
            display_name: "Cursor",
            skill_dir: home.join(".cursor/skills/lean-ctx"),
        },
        SkillTarget {
            agent_key: "codex",
            display_name: "Codex CLI",
            skill_dir: crate::core::home::resolve_codex_dir()
                .unwrap_or_else(|| home.join(".codex"))
                .join("skills/lean-ctx"),
        },
        SkillTarget {
            agent_key: "copilot",
            display_name: "GitHub Copilot",
            skill_dir: home.join(".copilot/skills/lean-ctx"),
        },
        SkillTarget {
            agent_key: "openclaw",
            display_name: "OpenClaw",
            skill_dir: home.join(".openclaw/skills/lean-ctx"),
        },
    ]
}

fn is_skill_agent_detected(agent_key: &str, home: &std::path::Path) -> bool {
    match agent_key {
        "claude" => {
            command_exists("claude")
                || crate::core::editor_registry::claude_mcp_json_path(home).exists()
                || crate::core::editor_registry::claude_state_dir(home).exists()
        }
        "cursor" => home.join(".cursor").exists(),
        "codex" => {
            let codex_dir =
                crate::core::home::resolve_codex_dir().unwrap_or_else(|| home.join(".codex"));
            codex_dir.exists() || command_exists("codex")
        }
        "copilot" => {
            home.join(".copilot").exists()
                || home.join(".copilot/mcp-config.json").exists()
                || command_exists("copilot")
        }
        "openclaw" => home.join(".openclaw").exists() || command_exists("openclaw"),
        _ => false,
    }
}

/// Install SKILL.md for a specific agent. Returns the installed path.
pub fn install_skill_for_agent(home: &std::path::Path, agent_key: &str) -> Result<PathBuf, String> {
    let targets = build_skill_targets(home);
    let target = targets
        .into_iter()
        .find(|t| t.agent_key == agent_key)
        .ok_or_else(|| format!("No skill target for agent '{agent_key}'"))?;

    let skill_path = target.skill_dir.join("SKILL.md");
    std::fs::create_dir_all(&target.skill_dir).map_err(|e| e.to_string())?;

    if skill_path.exists() {
        let existing = std::fs::read_to_string(&skill_path).unwrap_or_default();
        if existing == SKILL_TEMPLATE {
            return Ok(skill_path);
        }
    }

    crate::config_io::write_atomic_with_backup(&skill_path, SKILL_TEMPLATE)?;
    Ok(skill_path)
}

/// Install SKILL.md for all detected agents.
/// Returns `Vec<(display_name, was_new_or_updated)>`.
pub fn install_all_skills(home: &std::path::Path) -> Vec<(String, bool)> {
    let targets = build_skill_targets(home);
    let mut results = Vec::new();

    for target in &targets {
        if !is_skill_agent_detected(target.agent_key, home) {
            continue;
        }

        let skill_path = target.skill_dir.join("SKILL.md");
        let already_current = skill_path.exists()
            && std::fs::read_to_string(&skill_path).is_ok_and(|c| c == SKILL_TEMPLATE);

        if already_current {
            results.push((target.display_name.to_string(), false));
            continue;
        }

        if let Err(e) = std::fs::create_dir_all(&target.skill_dir) {
            tracing::warn!(
                "Failed to create skill dir for {}: {e}",
                target.display_name
            );
            continue;
        }

        match crate::config_io::write_atomic_with_backup(&skill_path, SKILL_TEMPLATE) {
            Ok(()) => results.push((target.display_name.to_string(), true)),
            Err(e) => {
                tracing::warn!("Failed to write SKILL.md for {}: {e}", target.display_name);
            }
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_rules_have_markers() {
        assert!(RULES_SHARED.contains(MARKER));
        assert!(RULES_SHARED.contains(END_MARKER));
        assert!(RULES_SHARED.contains(RULES_VERSION));
    }

    #[test]
    fn zed_rules_path_is_os_aware_and_matches_config_dir() {
        // Zed's config dir is platform-specific (macOS uses Application Support).
        // Rules must live under the SAME dir as the MCP config, never a hardcoded
        // ~/.config/zed on every OS (regression: rules missed on macOS).
        let home = std::path::Path::new("/home/tester");
        let zed = build_rules_targets(home, crate::core::config::RulesInjection::Shared)
            .into_iter()
            .find(|t| t.name == "Zed")
            .expect("Zed rules target must exist");
        let expected = crate::core::editor_registry::zed_config_dir(home).join("rules/lean-ctx.md");
        assert_eq!(zed.path, expected);
    }

    #[test]
    fn dedicated_rules_have_markers() {
        assert!(RULES_DEDICATED.contains(MARKER));
        assert!(RULES_DEDICATED.contains(END_MARKER));
        assert!(RULES_DEDICATED.contains(RULES_VERSION));
    }

    #[test]
    fn cursor_mdc_has_markers_and_frontmatter() {
        assert!(RULES_CURSOR_MDC.contains("lean-ctx"));
        assert!(RULES_CURSOR_MDC.contains(END_MARKER));
        assert!(RULES_CURSOR_MDC.contains(RULES_VERSION));
        assert!(RULES_CURSOR_MDC.contains("alwaysApply: true"));
    }

    #[test]
    fn shared_rules_contain_mode_selection() {
        assert!(RULES_SHARED.contains("Mode Selection"));
        assert!(RULES_SHARED.contains("full"));
        assert!(RULES_SHARED.contains("map"));
        assert!(RULES_SHARED.contains("signatures"));
        assert!(RULES_SHARED.contains("NEVER"));
    }

    #[test]
    fn shared_rules_has_never_native() {
        assert!(RULES_SHARED.contains("NEVER use native"));
        assert!(RULES_SHARED.contains("ctx_read"));
    }

    #[test]
    fn dedicated_rules_contain_modes() {
        assert!(RULES_DEDICATED.contains("auto"));
        assert!(RULES_DEDICATED.contains("full"));
        assert!(RULES_DEDICATED.contains("map"));
        assert!(RULES_DEDICATED.contains("signatures"));
        assert!(RULES_DEDICATED.contains("lines:N-M"));
        assert!(RULES_DEDICATED.contains("diff"));
    }

    #[test]
    fn dedicated_rules_has_proactive_section() {
        assert!(RULES_DEDICATED.contains("Proactive"));
        assert!(RULES_DEDICATED.contains("ctx_overview"));
        assert!(RULES_DEDICATED.contains("ctx_compress"));
    }

    #[test]
    fn cursor_mdc_contains_tool_mapping() {
        assert!(RULES_CURSOR_MDC.contains("Tool Mapping"));
        assert!(RULES_CURSOR_MDC.contains("ctx_read"));
        assert!(RULES_CURSOR_MDC.contains("ctx_search"));
        assert!(RULES_CURSOR_MDC.contains("Workflow"));
    }

    fn ensure_temp_dir() {
        let tmp = std::env::temp_dir();
        if !tmp.exists() {
            std::fs::create_dir_all(&tmp).ok();
        }
    }

    #[test]
    fn replace_section_with_end_marker() {
        ensure_temp_dir();
        let old = "user stuff\n\n# lean-ctx — Context Engineering Layer\n<!-- lean-ctx-rules-v2 -->\nold rules\n<!-- /lean-ctx -->\nmore user stuff\n";
        let path = std::env::temp_dir().join("test_replace_with_end.md");
        std::fs::write(&path, old).unwrap();

        let result = replace_markdown_section(&path, old).unwrap();
        assert!(matches!(result, RulesResult::Updated));

        let new_content = std::fs::read_to_string(&path).unwrap();
        assert!(new_content.contains(RULES_VERSION));
        assert!(new_content.starts_with("user stuff"));
        assert!(new_content.contains("more user stuff"));
        assert!(!new_content.contains("lean-ctx-rules-v2"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn replace_section_without_end_marker() {
        ensure_temp_dir();
        let old = "user stuff\n\n# lean-ctx — Context Engineering Layer\nold rules only\n";
        let path = std::env::temp_dir().join("test_replace_no_end.md");
        std::fs::write(&path, old).unwrap();

        let result = replace_markdown_section(&path, old).unwrap();
        assert!(matches!(result, RulesResult::Updated));

        let new_content = std::fs::read_to_string(&path).unwrap();
        assert!(new_content.contains(RULES_VERSION));
        assert!(new_content.starts_with("user stuff"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn append_to_shared_preserves_existing() {
        ensure_temp_dir();
        let path = std::env::temp_dir().join("test_append_shared.md");
        std::fs::write(&path, "existing user rules\n").unwrap();

        let result = append_to_shared(&path).unwrap();
        assert!(matches!(result, RulesResult::Injected));

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("existing user rules"));
        assert!(content.contains(MARKER));
        assert!(content.contains(END_MARKER));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn write_dedicated_creates_file() {
        ensure_temp_dir();
        let path = std::env::temp_dir().join("test_write_dedicated.md");
        if path.exists() {
            std::fs::remove_file(&path).ok();
        }

        let result = write_dedicated(&path, RULES_DEDICATED).unwrap();
        assert!(matches!(result, RulesResult::Injected));

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains(MARKER));
        assert!(content.contains("Mode Selection"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn write_dedicated_updates_existing() {
        ensure_temp_dir();
        let path = std::env::temp_dir().join("test_write_dedicated_update.md");
        std::fs::write(&path, "# lean-ctx — Context Engineering Layer\nold version").unwrap();

        let result = write_dedicated(&path, RULES_DEDICATED).unwrap();
        assert!(matches!(result, RulesResult::Updated));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn target_count() {
        let home = std::path::PathBuf::from("/tmp/fake_home");
        let targets = build_rules_targets(&home, crate::core::config::RulesInjection::Shared);
        assert_eq!(targets.len(), 25);
        // Dedicated mode swaps paths/formats but never changes the target count.
        let dedicated = build_rules_targets(&home, crate::core::config::RulesInjection::Dedicated);
        assert_eq!(dedicated.len(), 25);
    }

    #[test]
    fn dedicated_mode_swaps_shared_agents_to_dedicated_files() {
        use crate::core::config::RulesInjection;
        let home = std::path::Path::new("/home/tester");

        let shared = build_rules_targets(home, RulesInjection::Shared);
        let gemini_shared = shared.iter().find(|t| t.name == "Gemini CLI").unwrap();
        let opencode_shared = shared.iter().find(|t| t.name == "OpenCode").unwrap();
        assert!(matches!(gemini_shared.format, RulesFormat::SharedMarkdown));
        assert!(gemini_shared.path.ends_with("GEMINI.md"));
        assert!(matches!(
            opencode_shared.format,
            RulesFormat::SharedMarkdown
        ));
        assert!(opencode_shared.path.ends_with("AGENTS.md"));

        let dedicated = build_rules_targets(home, RulesInjection::Dedicated);
        let gemini = dedicated.iter().find(|t| t.name == "Gemini CLI").unwrap();
        let opencode = dedicated.iter().find(|t| t.name == "OpenCode").unwrap();
        // Never the user's shared instruction file in dedicated mode.
        assert!(matches!(gemini.format, RulesFormat::DedicatedMarkdown));
        assert_eq!(gemini.path, gemini_dedicated_rules_path(home));
        assert!(!gemini.path.ends_with("GEMINI.md"));
        assert!(matches!(opencode.format, RulesFormat::DedicatedMarkdown));
        assert_eq!(opencode.path, opencode_dedicated_rules_path(home));
        assert!(!opencode.path.ends_with("AGENTS.md"));
    }

    #[test]
    fn dedicated_session_summary_is_clean_and_agent_agnostic() {
        let s = dedicated_session_summary();
        assert!(s.contains("ctx_read"));
        assert!(s.contains("ctx_shell"));
        assert!(s.contains("ctx_search"));
        // Must not carry HTML markers or an @import pointer (Codex has no @import).
        assert!(!s.contains("<!--"));
        assert!(!s.contains('@'));
    }

    #[test]
    fn skill_template_not_empty() {
        assert!(!SKILL_TEMPLATE.is_empty());
        assert!(SKILL_TEMPLATE.contains("lean-ctx"));
    }

    #[test]
    fn skill_targets_count() {
        let home = std::path::PathBuf::from("/tmp/fake_home");
        let targets = build_skill_targets(&home);
        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn install_skill_creates_file() {
        ensure_temp_dir();
        let home = std::env::temp_dir().join("test_skill_install");
        let _ = std::fs::create_dir_all(&home);

        let fake_cursor = home.join(".cursor");
        let _ = std::fs::create_dir_all(&fake_cursor);

        let result = install_skill_for_agent(&home, "cursor");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, SKILL_TEMPLATE);

        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn install_skill_idempotent() {
        ensure_temp_dir();
        let home = std::env::temp_dir().join("test_skill_idempotent");
        let _ = std::fs::create_dir_all(&home);

        let fake_cursor = home.join(".cursor");
        let _ = std::fs::create_dir_all(&fake_cursor);

        let p1 = install_skill_for_agent(&home, "cursor").unwrap();
        let p2 = install_skill_for_agent(&home, "cursor").unwrap();
        assert_eq!(p1, p2);

        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn install_skill_unknown_agent() {
        let home = std::path::PathBuf::from("/tmp/fake_home");
        let result = install_skill_for_agent(&home, "unknown_agent");
        assert!(result.is_err());
    }

    #[test]
    fn match_agent_name_basic() {
        assert!(match_agent_name("cursor", "Cursor"));
        assert!(match_agent_name("opencode", "OpenCode"));
        assert!(match_agent_name("claude", "Claude Code"));
        assert!(match_agent_name("vscode", "VS Code"));
        assert!(match_agent_name("copilot", "Copilot CLI"));
        assert!(match_agent_name("kiro", "AWS Kiro"));
        assert!(match_agent_name("pi", "Pi Coding Agent"));
        assert!(match_agent_name("crush", "Crush"));
        assert!(match_agent_name("amp", "Amp"));
        assert!(match_agent_name("cline", "Cline"));
        assert!(match_agent_name("roo", "Roo Code"));
        assert!(match_agent_name("trae", "Trae"));
        assert!(match_agent_name("amazonq", "Amazon Q Developer"));
        assert!(match_agent_name("verdent", "Verdent"));
        assert!(match_agent_name("continue", "Continue"));
        assert!(match_agent_name("antigravity", "Antigravity"));
        assert!(match_agent_name("gemini", "Gemini CLI"));
        assert!(match_agent_name("augment", "Augment"));
        assert!(match_agent_name("openclaw", "OpenClaw"));
    }

    #[test]
    fn match_agent_name_no_false_positives() {
        assert!(!match_agent_name("cursor", "Claude Code"));
        assert!(!match_agent_name("opencode", "Cursor"));
        assert!(!match_agent_name("unknown_agent", "Cursor"));
    }

    #[test]
    fn inject_rules_for_agent_opencode() {
        ensure_temp_dir();
        let home = std::env::temp_dir().join("test_inject_rules_agent");
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::create_dir_all(&home);

        let opencode_dir = home.join(".config/opencode");
        let _ = std::fs::create_dir_all(&opencode_dir);

        let result = inject_rules_for_agent(&home, "opencode");
        assert!(
            !result.injected.is_empty() || !result.already.is_empty(),
            "should inject or find rules for OpenCode"
        );
        assert!(result.errors.is_empty(), "no errors expected");

        let agents_md = opencode_dir.join("AGENTS.md");
        if agents_md.exists() {
            let content = std::fs::read_to_string(&agents_md).unwrap();
            assert!(content.contains(RULES_VERSION));
        }

        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn inject_rules_for_agent_cursor() {
        ensure_temp_dir();
        let home = std::env::temp_dir().join("test_inject_rules_cursor");
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::create_dir_all(&home);

        let cursor_dir = home.join(".cursor");
        let _ = std::fs::create_dir_all(&cursor_dir);

        let result = inject_rules_for_agent(&home, "cursor");
        assert!(result.errors.is_empty(), "no errors expected");

        let mdc_path = home.join(".cursor/rules/lean-ctx.mdc");
        if mdc_path.exists() {
            let content = std::fs::read_to_string(&mdc_path).unwrap();
            assert!(content.contains(RULES_VERSION));
        }

        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn inject_rules_for_unknown_agent_is_empty() {
        let home = std::path::PathBuf::from("/tmp/fake_home_unknown");
        let result = inject_rules_for_agent(&home, "unknown_agent_xyz");
        assert!(result.injected.is_empty());
        assert!(result.updated.is_empty());
        assert!(result.already.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn write_dedicated_preserves_user_content_before_marker() {
        ensure_temp_dir();
        let path = std::env::temp_dir().join("test_dedicated_preserve_before.md");
        let old = format!(
            "# My custom rules\nDo not delete this!\n\n{MARKER}\n<!-- lean-ctx-rules-v2 -->\nold content\n{END_MARKER}"
        );
        std::fs::write(&path, &old).unwrap();

        let result = write_dedicated(&path, RULES_DEDICATED).unwrap();
        assert!(matches!(result, RulesResult::Updated));

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("My custom rules"),
            "user content before marker must be preserved"
        );
        assert!(
            content.contains("Do not delete this!"),
            "user content before marker must be preserved"
        );
        assert!(
            content.contains(RULES_VERSION),
            "new rules version must be present"
        );
        assert!(
            !content.contains("lean-ctx-rules-v2"),
            "old version must be replaced"
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn write_dedicated_preserves_user_content_after_marker() {
        ensure_temp_dir();
        let path = std::env::temp_dir().join("test_dedicated_preserve_after.md");
        let old = format!(
            "{MARKER}\n<!-- lean-ctx-rules-v2 -->\nold content\n{END_MARKER}\n\n# User's extra notes\nKeep this too!\n"
        );
        std::fs::write(&path, &old).unwrap();

        let result = write_dedicated(&path, RULES_DEDICATED).unwrap();
        assert!(matches!(result, RulesResult::Updated));

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("User's extra notes"),
            "user content after marker must be preserved"
        );
        assert!(
            content.contains("Keep this too!"),
            "user content after marker must be preserved"
        );
        assert!(
            content.contains(RULES_VERSION),
            "new rules version must be present"
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn write_dedicated_preserves_content_both_sides() {
        ensure_temp_dir();
        let path = std::env::temp_dir().join("test_dedicated_preserve_both.md");
        let old = format!(
            "BEFORE CONTENT\n\n{MARKER}\n<!-- lean-ctx-rules-v2 -->\nold\n{END_MARKER}\n\nAFTER CONTENT\n"
        );
        std::fs::write(&path, &old).unwrap();

        let result = write_dedicated(&path, RULES_DEDICATED).unwrap();
        assert!(matches!(result, RulesResult::Updated));

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("BEFORE CONTENT"));
        assert!(content.contains("AFTER CONTENT"));
        assert!(content.contains(RULES_VERSION));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn write_dedicated_no_user_content_uses_template_directly() {
        ensure_temp_dir();
        let path = std::env::temp_dir().join("test_dedicated_no_user.md");
        let old = format!("{MARKER}\n<!-- lean-ctx-rules-v2 -->\nold content\n{END_MARKER}");
        std::fs::write(&path, &old).unwrap();

        let result = write_dedicated(&path, RULES_DEDICATED).unwrap();
        assert!(matches!(result, RulesResult::Updated));

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            content, RULES_DEDICATED,
            "without user content, template should be written as-is"
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn write_dedicated_preserves_mdc_frontmatter() {
        ensure_temp_dir();
        let path = std::env::temp_dir().join("test_dedicated_mdc_frontmatter.mdc");
        let old = format!(
            "---\ndescription: custom\nglobs: **/*\nalwaysApply: true\n---\n\nUser preamble here\n\n{MARKER}\n<!-- lean-ctx-rules-v2 -->\nold\n{END_MARKER}\n"
        );
        std::fs::write(&path, &old).unwrap();

        let result = write_dedicated(&path, RULES_CURSOR_MDC).unwrap();
        assert!(matches!(result, RulesResult::Updated));

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("User preamble here"),
            "user preamble must be preserved"
        );
        assert!(
            content.contains("custom"),
            "user frontmatter description must be preserved"
        );
        assert!(content.contains(RULES_VERSION));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn inject_result_tracks_backed_up_files() {
        let result = InjectResult {
            backed_up: vec!["/tmp/test.md.bak".to_string()],
            ..Default::default()
        };
        assert_eq!(result.backed_up.len(), 1);
        assert!(std::path::Path::new(&result.backed_up[0])
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("bak")));
    }
}
