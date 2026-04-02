# Changelog

All notable changes to lean-ctx are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/).

## [2.14.4] — 2026-04-02

### Fixed
- **LEAN_CTX_DISABLED kill-switch now works end-to-end** — The shell hook (bash/zsh/fish/powershell) previously ignored `LEAN_CTX_DISABLED` entirely. Setting it to `1` bypassed compression in the Rust code but the shell aliases were still loaded, spawning a `lean-ctx` process for every command. Now: the `_lc()` wrapper short-circuits to `command "$@"` when `LEAN_CTX_DISABLED` is set (zero overhead), the auto-start guard skips alias creation, and `lean-ctx -c` does an immediate passthrough. Closes #42.
- **`lean-ctx-status` shows DISABLED state** — `lean-ctx-status` now prints `DISABLED (LEAN_CTX_DISABLED is set)` when the kill-switch is active.
- **Help text documents both env vars** — `--help` now shows `LEAN_CTX_DISABLED=1` (full kill-switch) and `LEAN_CTX_ENABLED=0` (prevent auto-start, `lean-ctx-on` still works).

## [2.14.3] — 2026-04-02

### Added
- **Full Output Tee** — New `tee_mode` config (`always`/`failures`/`never`) replaces the old `tee_on_error` boolean. When set to `always`, full uncompressed output is saved to `~/.lean-ctx/tee/` and referenced in compressed output. Backward-compatible: `tee_on_error: true` maps to `failures`. Use `lean-ctx tee last` to view the most recent log. Closes #2021.
- **Raw Mode** — Skip compression entirely with `ctx_shell(command, raw=true)` in MCP or `lean-ctx -c --raw <command>` on CLI. New `lean-ctx-raw` shell function in all hooks (bash/zsh/fish/PowerShell). Use for small outputs or when full detail is critical. Closes #2022.
- **Truncation Warnings** — When output is truncated during compression, a transparent marker shows exactly how many lines were omitted and how to get full output (`raw=true`). Prevents silent data loss — the #1 reason users leave competing tools.
- **`LEAN_CTX_DISABLED` env var** — Master kill-switch that bypasses all compression in both shell hook and MCP server. Set `LEAN_CTX_DISABLED=1` to pass everything through unmodified.
- **ANSI Auto-Strip** — ANSI escape sequences are automatically stripped before compression, preventing wasted tokens on invisible formatting codes. Centralized `strip_ansi` implementation replaces 3 duplicated copies.
- **Passthrough URLs** — New `passthrough_urls` config option. Curl commands targeting listed URLs skip JSON schema compression and return full response bodies. Useful for local APIs where full JSON is needed.
- **Zero Telemetry Badge** — README and comparison table now explicitly highlight lean-ctx's privacy-first design: zero telemetry, zero network requests, zero PII exposure.
- **User TOML Filters** — Define custom compression rules in `~/.lean-ctx/filters/*.toml`. User filters are applied before builtin patterns. Supports regex pattern matching with replacement and keep-lines filtering. New CLI: `lean-ctx filter [list|validate|init]`. Closes #2023.
- **PreToolUse Hook for Codex** — Codex CLI now gets PreToolUse-style hook scripts alongside AGENTS.md, matching Claude and Cursor/Gemini behavior. Closes #2024.
- **New AI Tool Integrations** — Added `opencode`, `aider`, and `amp` as supported agents. Use `lean-ctx init --agent opencode|aider|amp`. Total supported agents: 19. Closes #2026.
- **Discover Enhancement** — `lean-ctx discover` now shows a formatted table with per-command token estimates, USD savings projection (daily and monthly), and uses real compression stats when available. Shared logic between CLI and MCP tool. Closes #2025.

### Changed
- `ctx_shell` MCP tool schema now accepts `raw` boolean parameter.
- Server instructions include raw mode and tee file hints.
- Help text updated for new commands (`filter`, `tee last`, `-c --raw`).

## [2.14.2] — 2026-04-02

### Fixed
- **Shell hook quoting** — `git commit -m "message with spaces"` now works correctly. The `_lc()` wrapper previously used `$*` which collapsed quoted arguments into a flat string; fixed to use `$@` (bash/zsh), unquoted `$argv` (fish), and splatted `@args` (PowerShell) to preserve argument boundaries. Closes #41.
- **Terminal colors preserved** — Commands run through the shell hook in a real terminal (outside AI agent context) now inherit stdout/stderr directly, preserving ANSI colors, interactive prompts, and pager behavior. Previously, output was piped through a streaming buffer which caused child processes to disable color output (`isatty()` returned false). Closes #40.

### Removed
- `exec_streaming` mode — replaced by `exec_inherit_tracked` which passes output through unmodified while still recording command usage for analytics.

## [2.14.1] — 2026-04-02

### Autonomous Intelligence Layer

lean-ctx now runs its optimization pipeline **autonomously** — no manual tool calls needed.
The system self-configures, pre-loads context, deduplicates files, and provides efficiency hints
without the user or AI agent triggering anything explicitly.

### Added
- **Session Lifecycle Manager** — Automatically triggers `ctx_overview` or `ctx_preload` on the first MCP tool call of each session, delivering immediate project context
- **Related Files Hints** — After every `ctx_read`, appends `[related: ...]` hints based on the import graph, guiding the AI to relevant files
- **Silent Background Preload** — Top-2 imported files are automatically cached after each `ctx_read`, eliminating cold-cache latency on follow-up reads
- **Auto-Dedup** — When the session cache reaches 8+ files, `ctx_dedup` runs automatically to eliminate cross-file redundancy (measured: -89.5% in real sessions)
- **Task Propagation** — Session task context automatically flows to all `ctx_read` and `ctx_multi_read` calls for better compression targeting
- **Shell Efficiency Hints** — When `grep`, `cat`, or `find` run through `ctx_shell`, lean-ctx suggests the more token-efficient MCP equivalent
- **`AutonomyConfig`** — Full configuration struct with per-feature toggles and environment variable overrides (`LEAN_CTX_AUTONOMY=false` to disable all)
- **PHP/Laravel Support** — Full PHP AST extraction, Laravel-specific compression (Eloquent models, Controllers, Migrations, Blade templates), and `php artisan` shell hook patterns
- **15 new integration tests** for the autonomy layer (`autonomy_tests.rs`)

### Changed
- **System Prompt** — Replaced verbose `PROACTIVE` + `OTHER TOOLS` blocks with a compact `AUTONOMY` block, reducing cognitive load on the AI agent (~20 tokens saved per session)
- **`ctx_multi_read`** — Now accepts and propagates session task for context-aware compression

### Fixed
- **Version command** — `lean-ctx --version` now uses `env!("CARGO_PKG_VERSION")` instead of a hardcoded string

### Performance
- **Net savings: ~1,739 tokens/session** (analytical measurement)
- Pre-hook wrapper overhead: 10 tokens (one-time)
- Related hints: ~10 tokens per `ctx_read` call
- Silent preload savings: ~974 tokens (eliminates 2 manual reads)
- Auto-dedup savings: ~750 tokens at 15% reduction on typical cache
- System prompt delta: -20 tokens

### Configuration
All autonomy features are **enabled by default**. Disable individually or globally:
```toml
# ~/.lean-ctx/config.toml
[autonomy]
enabled = true
auto_preload = true
auto_dedup = true
auto_related = true
silent_preload = true
dedup_threshold = 8
```
Or via environment: `LEAN_CTX_AUTONOMY=false`

## [2.14.0] — 2026-04-02

### Intelligence Layer Architecture

lean-ctx transforms from a pure compressor into an Intelligence Layer between user, AI tool, and LLM.

### Added
- `ctx_preload` MCP tool — proactive context orchestration based on task + import graph
- L-Curve Context Reorder Engine — classifies lines into 7 categories, reorders for optimal LLM attention

### Changed
- Output-format reordering: file content first, metadata last
- IB-Filter 2.0 with empirical L-curve attention weights
- LLM-native encoding with 15+ token optimization rules
- System prompt cleanup (~200 wasted tokens removed)

### Fixed
- Shell hook compression broken when stdout piped
- Shell hook stats lost due to early `process::exit()`
