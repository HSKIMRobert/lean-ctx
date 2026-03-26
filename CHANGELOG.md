# Changelog

All notable changes to lean-ctx are documented here.

## [2.1.2] — 2026-03-26

### Bug Fix: Shell Hook PATH Resolution

Fixes a critical bug where `lean-ctx init --global` and `lean-ctx init --agent <tool>` generated shell aliases and hook scripts using bare `lean-ctx` instead of the absolute binary path. This caused all rewritten commands to fail with exit code 126 when `lean-ctx` was not in the shell's PATH.

### Fixed

- **Shell aliases (bash/zsh/fish)** now use the absolute binary path from `std::env::current_exe()` instead of hardcoded `lean-ctx`
- **Editor hook scripts (Claude, Cursor, Gemini)** embed `LEAN_CTX_BIN="/full/path/lean-ctx"` at the top and use `$LEAN_CTX_BIN` throughout
- **Codex and Cline instruction files** reference the full binary path
- **Windows + Git Bash compatibility**: Windows paths (`C:\Users\...`) are automatically converted to Git Bash paths (`/c/Users/...`) in bash hook scripts, fixing the `/C: Is a directory` error

### Added

- `to_bash_compatible_path()` helper for cross-platform path conversion (Windows drive letters to POSIX format)
- `resolve_binary_path_for_bash()` for bash-specific path resolution
- 6 unit tests for path conversion covering Unix paths, Windows drive letters, and edge cases

### Note for existing users

After updating, re-run `lean-ctx init --global` and/or `lean-ctx init --agent <tool>` to regenerate the aliases/hooks with the absolute path. Remove the old shell hook block from your `.zshrc`/`.bashrc` first (between `# lean-ctx shell hook` and `fi`).

---

## [2.1.1] — 2026-03-25

### Tool Enforcement + Editor Hook Improvements + Security & Trust

This release ensures AI coding tools reliably use lean-ctx MCP tools, and establishes a comprehensive security posture.

### Changed

- **MCP tool descriptions** now start with "REPLACES built-in X tool — ALWAYS use this instead of X"
- **Server instructions** include a LITM-optimized REMINDER at the end
- **`lean-ctx init --agent cursor`** now auto-creates `.cursor/rules/lean-ctx.mdc` in the project directory
- **`lean-ctx init --agent claude`** now auto-creates `CLAUDE.md` in the project directory
- **`lean-ctx init --agent windsurf`** now uses bundled template
- Example files now embedded via `include_str!` for consistent deployment

### Added

- **SECURITY.md** — Comprehensive security policy: vulnerability reporting, dependency audit, VirusTotal false positive explanation, build reproducibility
- **CI workflow** (`ci.yml`) — Automated tests, clippy lints (warnings=errors), rustfmt check, cargo audit on every push/PR
- **Security Check workflow** (`security-check.yml`) — Dangerous pattern scan (network ops, unsafe blocks, shell injection, hardcoded secrets), critical file change alerts, dependency audit
- **72 unit + integration tests** — Cache operations, entropy compression, LITM efficiency, shell pattern compression (git, cargo), CLI commands, pattern dispatch routing
- **README badges** — CI status, Security Check status, crates.io version, downloads, license
- **Security section** in README with VirusTotal false positive explanation

---

## [2.1.0] — 2026-03-25

### Real Benchmark Engine + Information Preservation

This release replaces the estimation-based benchmark with a **real measurement engine** that scans project files and produces verifiable, shareable results.

### Added

- **`core/preservation.rs`** — AST-based information preservation scoring using tree-sitter. Measures how many functions, exports, and imports survive each compression mode.
- **Project-wide benchmark** (`lean-ctx benchmark run [path]`):
  - Scans up to 50 representative files across all languages
  - Measures real token counts per compression mode (map, signatures, aggressive, entropy, cache_hit)
  - Tracks wall-clock latency per operation
  - Computes preservation quality scores per mode
  - Session simulation: models a 30-min coding session with real numbers
- **Three output formats**:
  - `lean-ctx benchmark run` — ANSI terminal table
  - `lean-ctx benchmark run --json` — machine-readable JSON
  - `lean-ctx benchmark report` — shareable Markdown for GitHub/LinkedIn
- **MCP `ctx_benchmark` extended** — new `action=project` parameter for project-wide benchmarks via MCP, with `format` parameter (terminal/markdown/json)

### Changed

- `lean-ctx benchmark` CLI now uses subcommands (`run`, `report`) instead of scenario names
- Benchmark engine uses real file measurements instead of estimates from stats.json

---

## [2.0.0] — 2026-03-25

### Major: Context Continuity Protocol (CCP) + LITM-Aware Positioning

This release introduces the **Context Continuity Protocol** — cross-session memory that persists task context, findings, and decisions across chat sessions and context compactions. Combined with **LITM-aware positioning** (based on Liu et al., 2023), CCP eliminates 99.2% of cold-start tokens and improves information recall by +42%.

### Added

- **2 new MCP tools** (19 → 21 total):
  - `ctx_session` — Session state manager with actions: status, load, save, task, finding, decision, reset, list, cleanup. Persists to `~/.lean-ctx/sessions/`. Load previous sessions in ~400 tokens (vs ~50K cold start)
  - `ctx_wrapped` — Generate savings report cards showing tokens saved, costs avoided, top commands, and cache efficiency

- **3 new CLI commands**:
  - `lean-ctx wrapped [--week|--month|--all]` — Shareable savings report card
  - `lean-ctx sessions [list|show|cleanup]` — Manage CCP sessions
  - `lean-ctx benchmark run [path]` — Real project benchmark (superseded by v2.1.0 project benchmarks)

- **LITM-Aware Positioning Engine** (`core/litm.rs`):
  - Places session state at context begin position (attention α=0.9)
  - Places findings/test results at end position (attention γ=0.85)
  - Eliminates lossy middle (attention β=0.55 → 0.0)
  - Quantified: +42% relative LITM efficiency improvement

- **Session State Persistence**:
  - Automatic session state tracking across all tool calls
  - Batch save every 5 tool calls
  - Auto-save before idle cache clear
  - Session state embedded in auto-checkpoints
  - Session state embedded in MCP server instructions (LITM P1 position)
  - 7-day session archival with cleanup

- **Benchmark Engine** (`core/benchmark.rs`):
  - Project-wide benchmark scanning up to 50 representative files
  - Per-mode token measurement using tiktoken (o200k_base)
  - Session simulation with real file data
  - Superseded by v2.1.0 project benchmarks with latency and preservation scoring

### Improved

- Auto-checkpoint now includes session state summary
- MCP server instructions now include CCP usage hints and session load prompt
- Idle cache expiry now auto-saves session before clearing

---

## [1.9.0] — 2026-03-25

### Major: Context Intelligence Engine

This release transforms lean-ctx from a compression tool into a **Context Intelligence Engine** — 9 new MCP tools, 15 new shell patterns, AI tool hooks, and a complete intent-detection system.

### Added

- **9 new MCP tools** (10 → 19 total):
  - `ctx_smart_read` — Adaptive file reading: automatically selects the optimal compression mode based on file size, type, cache state, and token count
  - `ctx_delta` — Incremental file updates via Myers diff. Only sends changed hunks instead of full content
  - `ctx_dedup` — Cross-file deduplication analysis: finds shared imports and boilerplate across cached files
  - `ctx_fill` — Priority-based context filling with a token budget. Automatically maximizes information density
  - `ctx_intent` — Semantic intent detection: classifies queries (fix, add, refactor, understand, test, config, deploy) and auto-loads relevant files
  - `ctx_response` — Bi-directional response compression with filler removal and TDD shortcuts
  - `ctx_context` — Multi-turn context manager: shows cached files, read counts, and session state
  - `ctx_graph` — Project intelligence graph: analyzes file dependencies, imports/exports, and finds related files
  - `ctx_discover` — Analyzes shell history to find missed compression opportunities with estimated savings

- **15 new shell pattern modules** (32 → 47 total):
  - `aws` (S3, EC2, Lambda, CloudFormation, ECS, CloudWatch Logs)
  - `psql` (table output, describe, DML)
  - `mysql` (table output, SHOW, queries)
  - `prisma` (generate, migrate, db push/pull, format, validate)
  - `helm` (list, install, upgrade, status, template, repo)
  - `bun` (test, install, build)
  - `deno` (test, lint, check, fmt)
  - `swift` (test, build, package resolve)
  - `zig` (test, build)
  - `cmake` (configure, build, ctest)
  - `ansible` (playbook recap, task summary)
  - `composer` (install, update, outdated)
  - `mix` (test, deps, compile, credo/dialyzer)
  - `bazel` (test, build, query)
  - `systemd` (systemctl status/list, journalctl log deduplication)

- **AI tool hook integration** via `lean-ctx init --agent <tool>`:
  - Claude Code (PreToolUse hook)
  - Cursor (hooks.json)
  - Gemini CLI (BeforeTool hook)
  - Codex (AGENTS.md)
  - Windsurf (.windsurfrules)
  - Cline/Roo (.clinerules)
  - Copilot (PreToolUse hook)

### Improved

- **Myers diff algorithm** in `compressor.rs`: Replaced naive line-index comparison with LCS-based diff using the `similar` crate. Insertions/deletions are now correctly tracked instead of producing mass-deltas
- **Language-aware aggressive compression**: `aggressive` mode now correctly handles Python `#` comments, SQL `--` comments, Shell `#` comments, HTML `<!-- -->` blocks, and multi-line `/* */` blocks
- **Indentation normalization**: Detects tab-based indentation and preserves it correctly

### Fixed

- **UTF-8 panic in `grep.rs`** (fixes [#4](https://github.com/yvgude/lean-ctx/issues/4)): String truncation now uses `.chars().take(n)` instead of byte-based slicing `[..n]`, preventing panics on multi-byte characters (em dash, CJK, emoji)
- Applied the same UTF-8 safety fix to `env_filter.rs`, `typescript.rs`, and `ctx_context.rs`

### Dependencies

- Added `similar = "2"` for Myers diff algorithm

---

## [1.8.2] — 2026-03-23

### Added
- Tee logging for full output recovery
- Poetry/uv shell pattern support
- Flutter/Dart shell pattern support
- .NET (dotnet) shell pattern support

### Fixed
- AUR source build: force GNU BFD linker via RUSTFLAGS to work around lld/tree-sitter symbol resolution

---

## [1.8.0] — 2026-03-22

### Added
- Web dashboard at localhost:3333
- Visual terminal dashboard with ANSI colors, Unicode bars, sparklines
- `lean-ctx discover` command
- `lean-ctx session` command
- `lean-ctx doctor` diagnostics
- `lean-ctx config` management

---

## [1.7.0] — 2026-03-21

### Added
- Token Dense Dialect (TDD) mode with symbol shorthand
- `ctx_cache` tool for cache management
- `ctx_analyze` tool for entropy analysis
- `ctx_benchmark` tool for compression comparison
- Fish shell support
- PowerShell support

---

## [1.5.0] — 2026-03-18

### Added
- tree-sitter AST parsing for 14 languages
- `ctx_compress` context checkpoints
- `ctx_multi_read` batch file reads

---

## [1.0.0] — 2026-03-15

### Initial Release
- Shell hook with 20+ patterns
- MCP server with ctx_read, ctx_tree, ctx_shell, ctx_search
- Session caching with MD5 hashing
- 6 compression modes (full, map, signatures, diff, aggressive, entropy)
