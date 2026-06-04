# Change Log

All notable changes to the **lean-ctx** VS Code extension are documented here.

## [0.1.1] — Unreleased

### Fixed

- **Session stats never loaded** (#347, thanks [@shawonis08](https://github.com/shawonis08)): the sidebar and status bar always showed zeros because `getSessionStats()` invoked `lean-ctx metrics --json`, a subcommand that does not exist, so every call threw and fell back to empty defaults. Stats are now sourced from `lean-ctx stats json` — the authoritative per-tool breakdown — mapping `commands.ctx_read` / `ctx_search` / `ctx_shell` counts and deriving tokens saved from the lifetime input/output totals, so reads, searches, shells and savings populate with real numbers.

## [0.1.0] — 2026-06-04

First public release on the VS Code Marketplace and Open VSX (Cursor, VSCodium, Windsurf).

### Added

- **Sidebar dashboard** — live token savings, session stats, and file activity.
- **Knowledge panel** — browse decisions, discoveries, blockers and insights from the current session.
- **Repo map** — the most relevant files in your project, ranked.
- **Semantic search** — search the codebase by meaning, with jump-to-result.
- **Status bar** — live token-savings counter with one-click dashboard access.
- **Setup & Doctor commands** — run `lean-ctx setup` / `lean-ctx doctor` from the command palette into a dedicated output channel.
- **Configure MCP for this workspace** — writes a `.vscode/mcp.json` stdio entry pointing at the resolved binary (existing servers preserved).
- **Open Web Dashboard** — launches `lean-ctx dashboard` in an integrated terminal.
- **Binary auto-detection** — finds `lean-ctx` on `PATH`, `~/.cargo/bin`, and Homebrew, so the extension works even when a GUI-launched editor inherits a stripped `PATH`.

### Security

- All CLI invocations use `spawn`/`execFileSync` with argument arrays — no shell interpolation.
- Webview escapes all dynamic values and verifies message origins (CodeQL-clean).
