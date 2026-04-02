# Changelog

All notable changes to lean-ctx are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/).

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
