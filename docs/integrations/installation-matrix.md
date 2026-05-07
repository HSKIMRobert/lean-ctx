# Installation Matrix (Setup / Init / Update)

This document defines the **exact** wiring lean-ctx performs for every supported IDE/agent and for every installation path.

## Installation paths (entry points)

- **`lean-ctx setup`** (recommended): detects installed IDEs/agents, picks a default `HookMode`, installs shell hook + rules + skills + hooks, and applies repairs (`--fix`) when needed.
- **`lean-ctx init --global`**: installs shell aliases/hook only (no IDE MCP wiring).
- **`lean-ctx init --agent <name> [--mode <mcp|cli-redirect|hybrid>]`**: installs IDE-specific hook/rules and (by default) configures **MCP**. For CLI-redirect, pass `--mode cli-redirect` (it will disable MCP instead of configuring it).
- **`lean-ctx update`**: updates the binary, then runs a non-interactive **setup refresh** (`setup --non-interactive --yes --fix`) so wiring stays consistent.

## Default modes (as of v3.5.1)

CLI-Redirect is only used when hooks verifiably intercept ALL tool types (bash + read + grep).
All other agents use Hybrid (MCP + hooks/rules) to ensure full Context OS features
(graph, knowledge, sessions, heatmap, intent detection) work in the background.

| Agent key | Default mode in `setup` | Rationale |
|----------|--------------------------|-----------|
| `cursor` | **CLI-redirect** | hooks.json intercepts Shell + Read + Grep |
| `codex` | **CLI-redirect** | All file ops go through Bash → single hook catches everything |
| `gemini` | **CLI-redirect** | BeforeTool hooks for shell + read_file + grep + list_dir |
| `claude` / `claude-code` | **Hybrid** | PreToolUse hooks don't fire in headless mode → needs MCP |
| `crush` | **Hybrid** | No hooks at all, rules only → MCP required for reliable path |
| `hermes` | **Hybrid** | No hooks, rules only → MCP required |
| `opencode` | **Hybrid** | Bash plugin only, no Read/Grep interception → MCP for reads |
| `pi` | **Hybrid** | External npm package routing, can't verify → MCP as fallback |
| `qoder` | **Hybrid** | Bash hook only, no Read/Grep → MCP for reads |
| `windsurf` | **Hybrid** | Shell available + MCP for full Context OS |
| `amp` | **Hybrid** | Amp supports MCP wiring; keep both paths |
| all others | **Hybrid** or **MCP** | MCP-first integration (extensions / no reliable shell) |

## What gets installed per agent (canonical files)

Legend:
- **MCP config**: editor/agent config file contains a `lean-ctx` server entry (tool schemas available to host).
- **MCP disabled**: any existing `lean-ctx` entry is removed from the config file.

| Agent | MCP config path | Rules path | Hooks/scripts | Skill |
|------|------------------|-----------|--------------|-------|
| Cursor (`cursor`) | `~/.cursor/mcp.json` (CLI-redirect: hooks only) | `~/.cursor/rules/lean-ctx.mdc` | `~/.cursor/hooks.json` + `~/.cursor/hooks/lean-ctx-*.sh` | `~/.cursor/skills/lean-ctx/SKILL.md` |
| Claude Code (`claude`) | `~/.claude.json` (MCP enabled — Hybrid) | `~/.claude/rules/lean-ctx.md` + `~/.claude/CLAUDE.md` | `~/.claude/settings.json` hook wiring (Bash rewrite + Read redirect) | `~/.claude/skills/lean-ctx/SKILL.md` |
| Codex (`codex`) | `~/.codex/config.toml` (CLI-redirect: hooks only) | `~/.codex/LEAN-CTX.md` + `~/.codex/AGENTS.md` | `~/.codex/hooks.json` (SessionStart/PreToolUse) | `~/.codex/skills/lean-ctx/SKILL.md` |
| OpenCode (`opencode`) | `~/.config/opencode/opencode.json` (MCP enabled — Hybrid) | `~/.config/opencode/rules/lean-ctx.md` | `~/.config/opencode/plugins/lean-ctx.ts` | — |
| Windsurf (`windsurf`) | `~/.codeium/windsurf/mcp_config.json` | `~/.codeium/windsurf/rules/lean-ctx.md` | project `.windsurfrules` (when not global) | — |
| VS Code / Copilot (`copilot`) | `~/Library/Application Support/Code/User/mcp.json` (macOS) | `~/Library/Application Support/Code/User/.../copilot-instructions.md` | — | — |
| JetBrains (`jetbrains`) | `~/.jb-mcp.json` (snippet file for copy/paste) | `~/.jb-rules/lean-ctx.md` | — | — |
| Cline (`cline`) | Cline MCP settings JSON | `~/.cline/rules/lean-ctx.md` | — | — |
| Roo (`roo`) | Roo MCP settings JSON | `~/.roo/rules/lean-ctx.md` | — | — |
| Kiro (`kiro`) | `~/.kiro/settings/mcp.json` | `~/.kiro/steering/lean-ctx.md` | — | — |
| Gemini (`gemini`) | `~/.gemini/settings.json` | `~/.gemini/GEMINI.md` | Gemini hooks (if present) | — |
| Antigravity (`antigravity`) | `~/.gemini/antigravity/mcp_config.json` | `~/.gemini/antigravity/rules/lean-ctx.md` | — | — |
| Crush (`crush`) | `~/.config/crush/crush.json` (MCP enabled — Hybrid) | `~/.config/crush/rules/lean-ctx.md` | — | — |
| Hermes (`hermes`) | `~/.hermes/config.yaml` (MCP enabled — Hybrid) | `~/.hermes/HERMES.md` or project `.hermes.md` | — | — |
| Amp (`amp`) | `~/.config/amp/settings.json` | `~/.ampcoder/rules/lean-ctx.md` | — | — |
| Pi (`pi`) | `~/.pi/agent/mcp.json` | `~/.pi/agent/rules/lean-ctx.md` | — | — |
| Qwen (`qwen`) | `~/.qwen/settings.json` | `~/.qwen/rules/lean-ctx.md` | — | — |
| Trae (`trae`) | `~/.trae/mcp.json` | `~/.trae/rules/lean-ctx.md` | — | — |
| Amazon Q (`amazonq`) | `~/.aws/amazonq/default.json` | `~/.aws/amazonq/rules/lean-ctx.md` | — | — |
| Verdent (`verdent`) | `~/.verdent/mcp.json` | `~/.verdent/rules/lean-ctx.md` | — | — |
| Zed | Zed settings JSON | `~/.config/zed/rules/lean-ctx.md` | — | — |

## Idempotency & repairs

- `setup --fix` and `update` are intended to be **safe and repeatable**:
  - Hybrid mode ensures MCP server entries are present in editor configs.
  - CLI-redirect mode (Cursor, Codex, Gemini only) relies on hooks exclusively.
  - Rules and skills are overwritten to the mode-correct versions.
  - Hook installation is merge-based where supported (preserves other hooks/plugins).

