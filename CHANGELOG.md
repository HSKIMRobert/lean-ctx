# Changelog

All notable changes to lean-ctx are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

## [3.5.15] — 2026-05-11

### Fixed

- **Dashboard "unauthorized" on localhost** — Users accessing the dashboard on `localhost` after v3.5.14 saw `/api/stats: unauthorized` because the browser didn't have the auth token. The server now auto-injects the token into HTML for loopback connections (`127.0.0.1`, `::1`) so the JS fetch interceptor can authenticate API calls automatically. API auth remains fully active — no bypass, no CSRF risk. Fixes webut's report.
- **Dashboard probe sends Bearer** — The `dashboard_responding` health probe now sends the saved Bearer token, so the "already running" detection works correctly with auth-enabled dashboards.
- **Large file crash / MCP hang** — Reading very large files (multi-GB) via `ctx_read` or `ctx_smart_read` caused the MCP server to allocate unbounded RAM and crash. Now enforced at 4 layers: binary file detection rejects before any I/O, `metadata().len()` checks reject before allocation, `read_file_lossy` refuses unbounded reads on `stat()` failure, and MCP dispatch returns `Err(ErrorData)` instead of `Ok("ERROR:...")` to prevent client retries. Fixes sb's report.

### Added

- **Binary file detection** (`core::binary_detect`) — Detects 100+ binary file extensions (Parquet, SQLite, ONNX, ZIP, images, ML models, bytecode, archives, fonts, disk images) plus magic-byte NULL check on the first 8 KB. Returns human-readable file type labels (e.g. "columnar data file", "ML model file"). Used across `ctx_read`, `ctx_smart_read`, `ctx_multi_read`, and `ctx_prefetch`.
- **Live Observatory event explanations** — Every event in the dashboard's Live Observatory now has a `?` help icon. Click to expand an inline explanation of what the event means and whether user action is needed. SLO violations ("violated · CompressionRatio") and compression events ("entropy_adaptive · 293 → 264 lines") are now clearly documented. Event type legend added to "How it works" section.
- **3 new security hardening tests** — `dashboard_api_auth_never_bypassed_for_loopback`, `dashboard_probe_sends_bearer_token`, loopback injection signature validation.

### Improved

- **Graceful error messages for binary/oversize files** — Instead of crashing or returning generic errors, binary files get a helpful message like "Binary file detected (.parquet, columnar data file). Use a specialized tool for this file type." Oversize files suggest `mode="lines:1-100"` for partial reads.
- **MCP error semantics** — Binary/oversize file errors now return `Err(ErrorData::invalid_params(...))` at the MCP dispatch level, signaling to clients that retrying won't help. Previously returned `Ok("ERROR: ...")` which caused some clients to retry indefinitely.

## [3.5.14] — 2026-05-10

### Performance

- **BLAKE3 hashing** — Replaced all MD5 (`md5_hex`, `md5_hex_bytes`) with BLAKE3 via centralized `core::hasher` module. 12 duplicate hash functions eliminated across the codebase. BLAKE3 is ~3x faster than MD5 for large inputs with better collision resistance.
- **Tree-sitter Query Cache** — Compiled tree-sitter `Query` objects are now cached in `OnceLock<HashMap>` statics in `chunks_ts`, `signatures_ts`, and `deep_queries`. Eliminates re-compilation of query patterns on every file parse. Parser instances reuse via `thread_local!`.
- **Token cache upgrade** — Token cache enlarged from 256→2048 entries with BLAKE3-based keys and LRU-like eviction (half-evict instead of full clear). Reduces redundant BPE tokenization across sessions.
- **SQLite Property Graph optimized** — Added `PRAGMA cache_size = -8000`, `mmap_size = 256MB`, `temp_store = MEMORY`. 5 new composite indices on `nodes(kind)`, `nodes(kind, file_path)`, `edges(kind)`, `edges(source_id, kind)`, `edges(target_id, kind)`. `busy_timeout(5000ms)` for WAL contention.
- **Parallel indexing** — `rayon::par_iter` for CPU-bound deep-query parsing in `ctx_impact build` (embeddings feature path).
- **ModePredictor Arc** — `ModePredictor` stored as `Arc<ModePredictor>` to avoid deep cloning on every `ctx_read` call.
- **Compact JSON serialization** — `ProjectIndex::save()` uses `serde_json::to_string` (compact) instead of `to_string_pretty`, reducing index file size and serialization time.
- **Server dispatch deduplicated** — `count_tokens` called once per request instead of redundantly after terse pass when content unchanged.

### Improved

- **Rules: Mode Selection Decision Tree** — Adopted community-contributed improvement (credit: Zeel Connor). Rules now include a numbered decision tree for `ctx_read` mode selection and an anti-pattern warning against using `full` for context-only files. Applied across all rule formats (shared, dedicated, Cursor MDC, CLI-redirect).
- **Flaky test fixes** — BM25 tests (`save_writes_project_root_marker`, `max_bm25_cache_bytes_reads_env`) now acquire `test_env_lock()` to prevent `env::set_var` race conditions. ContextBus tests use isolated temp SQLite databases via `test_bus()` instead of shared global DB.

### Added

- **`core::hasher` module** — Centralized BLAKE3 hashing: `hash_hex(bytes)`, `hash_str(s)`, `hash_short(s)`. Single source of truth for all non-cryptographic hashing.
- **`core::community` module** — Louvain-based community detection on the Property Graph (file clustering by dependency).
- **`core::pagerank` module** — PageRank computation on the Property Graph for file importance scoring.
- **`core::smells` module** — Code smell detection (long functions, deep nesting, high complexity).
- **`ctx_smells` tool** — MCP + CLI tool for code smell analysis with graph-enriched scoring.
- **58 MCP tools** — Up from 57 in previous release (added `ctx_smells`).

## [3.5.13] — 2026-05-10

### Fixed

- **Instruction files no longer compressed** — SKILL.md, AGENTS.md, RULES.md, .cursorrules, and files in `/skills/`, `/.cursor/rules/`, `/.claude/rules/` are now **always delivered in full mode**, bypassing all heuristic/bandit/adaptive mode selection. This was the root cause of agents losing instructions after v3.4.7 when the Intent Router was introduced. Guards added in 5 code paths: `resolve_auto_mode`, `predict_from_defaults`, `select_mode_with_task`, `auto_degrade_read_mode`, and CLI `read_cmd`. Fixes #159 regression, resolves GlemSom's report.
- **Markdown files exempt from aggressive compression** — `.md`, `.mdx`, `.txt`, `.rst` files no longer fall into the `aggressive` default bucket in `predict_from_defaults`. These file types return `None` (= full mode) to prevent stripping prose/instruction content.
- **Windows Claude Code PowerShell compatibility** — Claude Code hook matchers now include `PowerShell|powershell` on Windows, so PreToolUse hooks fire regardless of whether Claude uses Bash or PowerShell. Rewrite script also accepts PowerShell tool names. Fixes #192.

### Added

- **`is_instruction_file()` public API** — Reusable guard function detecting instruction/skill/rule files by filename and path patterns. Used across MCP, CLI, and server dispatch paths.
- **Lean4 formal proofs** — Theorems 12-13 in `ReadModes.lean`: instruction files always resolve to full mode, content is always preserved.
- **7 new regression tests** — `instruction_file_detection`, `resolve_auto_mode_returns_full_for_instruction_files`, `defaults_never_compress_markdown`, and PowerShell hook matcher tests.

## [3.5.12] — 2026-05-09

### Improved

- **RAM optimization: eliminate double tokenization** — `extract_chunks` in `bm25_index.rs`, `artifact_index.rs`, and `chunks_ts.rs` no longer allocates a `tokens: Vec<String>` per chunk. Token count is computed inline; the vector is set to `Vec::new()`. `add_chunk` tokenizes from `content` once for the inverted index and overwrites `token_count` from the fresh result. This eliminates one redundant allocation + tokenization pass per chunk during index build.
- **MemoryProfile fully wired** — The `MemoryProfile` enum (`low` / `balanced` / `performance`) now actively controls runtime behavior:
  - `max_bm25_cache_bytes()` respects profile limits (64 / 128 / 512 MB), with explicit user config taking precedence.
  - Semantic cache (`SemanticCacheIndex`) is skipped entirely when `memory_profile = low`.
  - Embedding engine loading is skipped in `ctx_semantic_search` and `ctx_knowledge` when `memory_profile = low`.
- **Doctor shows active memory profile** — `lean-ctx doctor` now displays the effective memory profile (low / balanced / performance), its source (env / config / default), and what it controls (cache limits, embedding status). Helps users understand and debug RAM behavior.
- **MCP manifest regenerated** — Updated `mcp-tools.json` to reflect current tool count (57 granular tools).

## [3.5.11] — 2026-05-09

### Fixed

- **Cache-loop elimination for hybrid-mode agents** — When an agent reads a file with `mode=auto` (compressed) and then re-reads with `mode=full`, the full content is now delivered immediately instead of returning a 2-line "already in context" stub. Previously, agents (especially smaller/local models) needed 3 calls to get full content: auto → full (stub) → fresh. A new `full_content_delivered` flag on cache entries tracks whether uncompressed content was already sent for the current hash.
- **Cache stub text no longer provokes unnecessary calls** — The "file already in context" message no longer suggests `fresh=true`, which misled weaker models into making a redundant third call. New text: "File content unchanged since last read (same hash). Already in your context window."
- **AGENTS.md Pi-header replaced on non-Pi agents** — When a project had `AGENTS.md` from a prior `lean-ctx init --agent pi` but was later initialized for OpenCode or another agent, the Pi-specific header ("CLI-first Token Optimization for Pi") persisted. The generic lean-ctx block now replaces it automatically.
- **Doctor check count mismatch (16/15)** — The daemon health check incremented `passed` but was not counted in `effective_total`, causing the summary to show e.g. "16/15 checks passed". Fixed by including the daemon check in the total (`+5` instead of `+4`).
- **"INDEXING IN PROGRESS" no longer blocks read output** — When the graph index is still building, the autonomy pre-hook returned the indexing notice as auto-context, which was prepended to the actual tool output. This is now suppressed — the file content is returned immediately while indexing continues in the background.

### Improved

- **RAM usage reduced during compaction/checkpoint** — Four targeted optimizations to prevent memory spikes reported during OpenCode session compaction:
  - **Codebook uses borrows instead of clones** — `build_from_files` now accepts `&[(&str, &str)]` instead of `Vec<(String, String)>`, eliminating a full duplication of all cached file contents (~2MB saved at 500k tokens).
  - **Auto-checkpoint skips signature extraction** — Periodic checkpoints now use `include_signatures: false`, avoiding expensive tree-sitter parsing. Explicit `ctx_compress` calls still extract signatures.
  - **Compressed output variants capped at 3 per cache entry** — Prevents unbounded growth of the `compressed_outputs` HashMap.
  - **Codebook early-exit at >50,000 lines** — Skips the codebook deduplication phase entirely for very large caches, preventing HashMap/HashSet memory explosions.

## [3.5.10] — 2026-05-09

### Added

- **4-layer terse compression engine** — Scientifically grounded compression pipeline replacing the legacy `output_density` / `terse_agent` settings with a unified `CompressionLevel` system (`off` / `lite` / `standard` / `max`):
  - **Layer 1 — Deterministic Output Terse** (`engine.rs`): Surprisal scoring, content/function-word filtering, filler-line removal, and a quality gate that preserves all paths and identifiers.
  - **Layer 2 — Pattern-Aware Residual** (`residual.rs`): Runs after pattern compression, applies terse on the remaining output with attribution split.
  - **Layer 3 — Agent Output Shaping** (`agent_prompts.rs`): Scale-aware brevity prompts injected into LLM instructions — telegraph-English-inspired format for `max`, dense atomic facts for `standard`, concise bullets for `lite`.
  - **Layer 4 — MCP Description Terse** (`mcp_compress.rs`): Compresses tool descriptions and lazy-load stubs for reduced schema overhead.
- **Unified `CompressionLevel` configuration** — Single `compression_level` setting in `config.toml` replaces the legacy `output_density` and `terse_agent` options. Resolution order: `LEAN_CTX_COMPRESSION` env var → `compression_level` config → legacy fallback. CLI: `lean-ctx compression <off|lite|standard|max>` (alias: `lean-ctx terse`).
- **Quality gate for terse compression** (`quality.rs`) — Ensures all file paths and code identifiers survive compression. If `max` level fails the quality check, automatically falls back to `standard`. Inputs shorter than 5 lines skip compression entirely.
- **Agent prompt injection across all IDEs** (`rules_inject.rs`) — Compression prompts are automatically injected into 7 agent rules files (Cursor `.cursorrules`, `~/.cursor/rules/lean-ctx.mdc`, Claude `.claude/rules/lean-ctx.md`, AGENTS.md, CRUSH, Qoder, Kiro). Injection runs from `lean-ctx compression`, `lean-ctx setup`, and on MCP server startup — ensuring retroactive consistency when users change settings.
- **Context Proof V2** (`context_proof_v2.rs`) — Proof-carrying context with claim extraction, quality levels Q0–Q4, and structured verification output.
- **Claim extractor** (`claim_extractor.rs`) — Decomposes session context into atomic verifiable claims for the proof system.
- **29 new Lean4 formal proofs** — Two new proof modules bringing the total to **82 machine-checked theorems** with zero `sorry`:
  - `TerseQuality.lean` (12 theorems): Quality gate correctness, conjunction semantics, idempotence, empty-set triviality.
  - `TerseEngine.lean` (17 theorems): Compression level ordering, Max-to-Standard fallback correctness, structural marker preservation, filter-subset invariant, high-score line protection.
- **Terse evaluation harness** (`terse_eval.rs`) — Integration test covering git diff, JSON API, Docker build, Cargo build, and Rust error outputs across all compression levels.
- **Domain-aware dictionaries** (`dictionaries.rs`) — Whole-word replacement dictionaries for general programming terms, Git operations, and domain-specific abbreviations. Applied after quality gate to prevent identifier corruption.
- **Surprisal-based line scoring** (`scoring.rs`) — Information-theoretic scoring using bigram surprisal to identify high-information-density lines for preservation.

### Improved

- **Dashboard: shared utilities refactored** — New `shared.js` library with common dashboard utilities, reducing code duplication across cockpit components.
- **Dashboard: cockpit components polished** — Updated Context Explorer, Agent Sessions, Graph Visualizer, Knowledge Base, Memory Inspector, Compression Stats, and Overview with improved layouts, consistent styling, and better data presentation.
- **Setup flow consolidated** — Premium feature configuration (compression, TDD) unified into a single interactive prompt flow. Shell alias refresh integrated.
- **Test suite robustness** — `terse_agent_tests.rs` rewritten to explicitly control both `LEAN_CTX_COMPRESSION` and `LEAN_CTX_TERSE_AGENT` env vars, eliminating dependency on local config state. Mutex poison recovery added. 5 new tests for the `CompressionLevel` system alongside 6 fixed legacy backward-compat tests.
- **Intensive benchmarks updated** — `intensive_benchmarks.rs` now benchmarks the new 4-layer terse pipeline instead of the removed `protocol::compress_output`.

### Fixed

- **Token counter overflow** (`counter.rs`) — `savings_pct` no longer panics when dictionary replacements expand text beyond the original token count.
- **Short input over-compression** — Inputs shorter than 5 lines are now passed through unchanged, preventing the terse engine from removing single-line outputs like file reads.
- **Legacy pipeline cleanup** — Removed deprecated `compress_output`, `OutputDensity` functions from `protocol.rs`. All compression now routes through the unified terse pipeline.

## [3.5.9] — 2026-05-09

### Fixed

- **Codex config corruption with tool approval entries (GitHub #191)** — When Codex auto-adds per-tool approval entries (`[mcp_servers.lean-ctx.tools.ctx_read]`, etc.) to `config.toml`, the parent `[mcp_servers.lean-ctx]` section could be missing (e.g. after a v3.5.6 upgrade removed it). `upsert_codex_toml` now detects orphaned `[mcp_servers.lean-ctx.*]` sub-tables and inserts the parent section **before** them instead of appending at the end, which Codex's TOML parser rejected with "invalid transport".
- **AGENTS.md reference uses absolute path** — The lean-ctx block in `~/.codex/AGENTS.md` now references `` `~/.codex/LEAN-CTX.md` `` instead of `LEAN-CTX.md (same directory)`, preventing AI agents from misinterpreting the relative reference as the project working directory.

### Security

- **fast-uri 3.1.0 → 3.1.2 (VSCode extension)** — Fixes GHSA-v39h-62p7-jpjc (malformed fragment decoding) and GHSA-q3j6-qgpj-74h6 (URI parsing vulnerability).

### Improved

- **Dashboard cockpit polish** — Refined Context Explorer with improved layout, resizable panels, and better file tree navigation. Updated styling across all cockpit components for consistency. Improved graph visualization layout and memory inspector presentation.

## [3.5.8] — 2026-05-08

### Security

- **CodeQL #40 (High): XSS in dashboard search** — `cockpit-search.js` fallback `esc()` function was `function(s) { return String(s); }` — no HTML escaping. Replaced with safe `textContent`→`innerHTML` implementation matching `format.js`.
- **CodeQL #38/#39 (Medium): Unpinned GitHub Actions** — `codecov/codecov-action@v4` and `EmbarkStudios/cargo-deny-action@v2` are now pinned to commit SHAs (`b9fd7d16…`, `5bb39ff5…`) in `ci.yml`.

### Fixed

- **Codex config corruption on mode change (GitHub #189)** — When `lean-ctx setup` or `lean-ctx update` ran with v3.5.6 (where Codex was CLI-Redirect mode), `remove_codex_toml_section` removed the `[mcp_servers.lean-ctx]` parent section but left orphaned sub-tables like `[mcp_servers.lean-ctx.env]`, causing Codex to fail with "invalid transport in mcp_servers.lean-ctx".
  - `remove_codex_toml_section` now removes **all** TOML sub-tables via prefix matching when removing a parent section.
  - `ensure_codex_mcp_server` now detects orphaned sub-tables and inserts the parent section **before** them instead of appending at the end.
  - `ensure_codex_mcp_server` now uses `toml_quote_value` for Windows backslash-safe TOML quoting (was using raw `format!` with double quotes).

## [3.5.7] — 2026-05-08

### Security

- **BM25 index memory balloon fix (GitHub #188)** — Oversized BM25 cache files (observed up to 50 GB in monorepos with vendor/generated code) could cause the daemon to allocate unbounded memory on startup, leading to system-wide swapping and OOM conditions. This release implements an 8-layer defense:
  1. **Load-time size guard** — `BM25Index::load()` now checks file metadata before reading. Indexes exceeding the configurable limit (default 512 MB) are quarantined by renaming to `.quarantined` and skipped.
  2. **Save-time size guard** — `BM25Index::save()` refuses to persist serialized data exceeding the limit, preventing bloated indexes from being written in the first place.
  3. **Chunk count warning** — Indexes with >50,000 chunks trigger a `tracing::warn` suggesting `extra_ignore_patterns` in `config.toml`.
  4. **Default vendor/build ignores** — 14 glob patterns (`vendor/**`, `dist/**`, `build/**`, `.next/**`, `__pycache__/**`, `*.min.js`, `*.bundle.js`, etc.) are now excluded from BM25 indexing by default.
  5. **File count cap** — `list_code_files()` stops collecting after 5,000 files per project, preventing runaway indexing in massive repos.
  6. **Configurable limit** — New `bm25_max_cache_mb` setting in `config.toml` (default: 512). Override per-project or via `LEAN_CTX_BM25_MAX_CACHE_MB` env var.
  7. **Project root marker** — `save()` writes a `project_root.txt` file alongside each index, enabling orphan detection when the original project directory is deleted.
  8. **`lean-ctx doctor` BM25 health check** — Doctor now scans all vector directories, warns about large indexes (>100 MB), and fails for oversized indexes. `lean-ctx doctor --fix` automatically prunes quarantined, oversized, and orphaned caches.

### Fixed

- **Codex integration mode changed from CLI-Redirect to Hybrid** — Codex exists in three variants (CLI, Desktop App, Cloud Agent) that share `~/.codex/config.toml`. Only the CLI variant has reliable shell hooks; Desktop and Cloud require MCP. lean-ctx now treats Codex as **Hybrid** (MCP + CLI hooks where available) instead of CLI-Redirect, ensuring all three variants work correctly.
- **Codex hook installer now writes MCP server entry** — `lean-ctx init --agent codex` now ensures `[mcp_servers.lean-ctx]` exists in `~/.codex/config.toml`. Previously, only CLI hooks and `codex_hooks = true` were written, leaving Desktop/Cloud variants without MCP access.
- **Codex LEAN-CTX.md upgrade detection** — `install_codex_instruction_docs()` now compares file content instead of just checking for the string "lean-ctx". This ensures the instruction file is updated when the template changes (e.g., CLI-only → Hybrid mode), instead of being silently skipped on every subsequent install.
- **Dashboard HTTP parser handles large POST bodies** — The dashboard TCP handler now reads complete HTTP messages using `Content-Length` header parsing instead of assuming the entire request fits in the first read. POST requests to API endpoints (e.g., knowledge CRUD, memory management) no longer fail silently when the body exceeds 8 KB. Maximum message size enforced at 2 MB.

### Added

- **Cockpit dashboard (complete rewrite)** — The localhost dashboard has been rebuilt from scratch as a modular single-page application:
  - **12 Web Components**: Overview, Live Activity, Context Explorer, Knowledge Base, Graph Visualizer, Agent Sessions, Memory Inspector, Compression Stats, Health Monitor, Search, Remaining Token Budget, Navigation.
  - **Modular Rust backend**: Monolithic route handler (~1,200 lines) replaced with 10 focused route modules (`routes/agents.rs`, `context.rs`, `graph.rs`, `knowledge.rs`, `memory.rs`, `stats.rs`, `system.rs`, `tools.rs`, `helpers.rs`, `mod.rs`).
  - **Shared JS libraries**: `api.js` (fetch wrapper with token auth), `charts.js` (SVG charting), `format.js` (number/byte/duration formatting), `router.js` (hash-based SPA routing), `shared.js` (common utilities).
  - **Full CSS redesign**: 800+ lines of modern CSS with dark theme, responsive layout, data tables, card grids, and chart containers.
  - Legacy dashboard preserved at `/legacy` route for backwards compatibility.
- **`lean-ctx cache prune` command** — New CLI command to scan `~/.lean-ctx/vectors/`, remove quarantined (`.quarantined`) files, oversized indexes, and orphaned directories (project root no longer exists). Reports count and freed space.
- **`lean-ctx doctor` BM25 cache health check** — Proactive diagnostics for BM25 index health, integrated into the standard doctor report. `--fix` auto-prunes.

### Improved

- **Codex instruction docs now document Hybrid mode** — `~/.codex/LEAN-CTX.md` now includes both MCP tool table (ctx_read, ctx_shell, ctx_search, ctx_tree) and CLI fallback instructions, with guidance on when to use which path depending on the Codex variant.
- **Website: Codex moved to Hybrid in Context OS table** — All 11 locale files and the ContextOsPage agent table updated. Codex now correctly appears under Hybrid mode instead of CLI-Redirect.
- **Website: Codex editor guide updated** — DocsGuideEditorsPage now describes Codex as running in Hybrid mode across CLI, Desktop, and Cloud variants.

## [3.5.6] — 2026-05-08

### Fixed

- **Daemon auto-restart on setup and update** — `lean-ctx setup` and `lean-ctx update` now automatically stop and restart the daemon with the current binary. Previously, a running daemon would be left untouched, causing stale-binary mismatches after updates. Both interactive and non-interactive (`--yes`) flows are covered.
- **Proactive stale daemon cleanup** — `is_daemon_running()` now removes orphaned PID and socket files when the referenced process is dead. This prevents connection attempts to stale Unix Domain Sockets after crashes or reboots.
- **UDS connection timeouts** — All daemon socket connections now have a 3-second connect timeout and 10-second I/O timeout. Previously, connections to a stale or unresponsive socket could block indefinitely, cascading into system-wide hangs.
- **Daemon readiness wait reduced** — The CLI auto-start readiness loop was reduced from 12 seconds to 3 seconds, keeping CLI commands responsive even when the daemon is slow to start.

### Improved

- **Website navigation completeness** — Added `/docs/concepts/multi-agent` to the Docs mega dropdown. Mobile navigation now includes all Context OS pages (Integrations, Shared Sessions, Context Bus, SDK) that were previously desktop-only.
- **Daemon documentation updated** — Integrations pillar and Context OS overview pages now document auto-restart on update, stale-file cleanup, and connection timeouts across all 11 languages.

## [3.5.5] — 2026-05-08

### Fixed

- **Search command compression blocked by auth-flow false positive** — `rg`, `grep`, `find`, `fd`, `ag`, and `ack` outputs were silently skipped by the compression pipeline whenever the search results contained OAuth-related strings (`device_code`, `user_code`, `verification_uri`, etc.) anywhere in the matched source code. This caused 0% savings for any `rg` search over a codebase that implements or references OAuth device-code flows — even though the output was search results, not an actual auth prompt. The fix skips the `contains_auth_flow` guard for search commands in both the CLI (`shell/compress.rs`) and MCP (`ctx_shell`) paths. Real auth flows (e.g. `az login`, `gh auth login`) are still preserved verbatim for non-search commands. Reported by aguarella (Discord).
- **Central `shorter_only` guard for all shell patterns** — Added a centralized length check in `patterns/mod.rs` that wraps every compressor (`FilterEngine`, `try_specific_pattern`, `json_schema`, `log_dedup`, `test`). No pattern can return `Some(result)` unless `result` is strictly shorter than the original output. Eliminates a class of bugs where patterns claimed compression without actually reducing size.
- **`grep` compressor removes verbatim threshold** — Removed the `<= 100 lines` early return that passed small `rg`/`grep` outputs through uncompressed. All search outputs are now grouped by file with per-file match limits, regardless of size. Combined with the `shorter_only` guard, small outputs that can't be meaningfully compressed correctly return `None` instead of faking 0% savings.
- **`gh` CLI verbatim returns replaced with `None`** — `gh pr diff`, `gh api`, `gh search`, `gh workflow`, and unknown `gh` subcommands no longer return `Some(output.to_string())` (which falsely claimed compression). They now return `None`, allowing fallback compressors or the caller to handle the output appropriately.
- **`safeguard_ratio` aligned with CLI behavior** — The MCP compression guard now uses a 5% floor only for small outputs (<2,000 tokens) and allows aggressive compression for large outputs, matching the CLI pipeline behavior.
- **`ctx_shell` search command inflation guard** — For search commands (`rg`, `grep`, etc.), the MCP handler now explicitly checks `c.len() <= output.len()` before using the compressed result, preventing any inflation from reaching the agent.
- **Codex `AGENTS.md` overwrite** — `install_codex_instruction_docs` now uses marked-block insertion (`<!-- lean-ctx -->...<!-- /lean-ctx -->`) instead of overwriting `~/.codex/AGENTS.md`, preserving user instructions. Reported by Vitu (Discord).

### Added

- **Knowledge CLI: export/import/remove** — Full CLI parity with MCP `ctx_knowledge`:
  - `lean-ctx knowledge export [--format json|jsonl|simple] [--output <path>]`
  - `lean-ctx knowledge import <path> [--merge replace|append|skip-existing] [--dry-run]`
  - `lean-ctx knowledge remove --category <cat> --key <key>`
  - Core: `import_facts()` with merge strategies, `export_simple()` for interop, `parse_import_data()` with auto-format detection.
  - Context OS: knowledge `import` events tracked via `KnowledgeRemembered` bus event.
- **Context OS optimizations** — Connection pooling for Context Bus R/W, broadcast channels replacing mutex-guarded Vec, inverted token index for BM25 search, LRU session eviction, metrics consolidation cleanup.

### Fixed (cont.)

- **Dashboard scroll after fullscreen** — `switchView()` now closes any active fullscreen before tab transitions, restoring scroll in all views. (GitHub #186)

## [3.5.4] — 2026-05-07

### Fixed

- **`gh` CLI compression safety** — Unknown `gh` subcommands (`gh pr diff`, `gh api`, `gh search`, `gh workflow`, `gh auth`, `gh secret`, etc.) now pass through verbatim instead of being truncated to 10 lines. Previously, fallback compressors (JSON, log-dedup) could also strip content from `gh api` and `gh search` output. The fix returns `Some(output)` for unmatched commands (blocking fallback compression), matching the safe behavior already used by `git` and `glab` patterns.
- **Uninstall proxy cleanup** — `lean-ctx uninstall` now cleans up Claude Code (`ANTHROPIC_BASE_URL` in `settings.json`) and Codex CLI (`OPENAI_BASE_URL` in `config.toml`) proxy settings. Previously only shell exports (Gemini) were removed, leaving Claude/Codex pointing at the dead local proxy after uninstall. If a saved upstream exists, Claude Code settings are restored to the original URL.
- **CLI `ls`/`grep` daemon path resolution** — `lean-ctx ls .` and `lean-ctx grep <pattern> .` now resolve relative paths to absolute before sending to the daemon, fixing incorrect directory listings when the daemon's CWD differs from the CLI's CWD.

### Added

- **Context Bus v2: Multi-Agent Coordination** — Major upgrade to the event bus with versioned events, causal lineage, consistency levels, and multi-agent conflict detection.
  - **Event versioning**: Every event now carries a monotonic `version` per (workspace, channel) and an optional `parentId` for causal chains.
  - **Consistency levels**: Events classified as `local` (informational), `eventual` (shared, async), or `strong` (requires sync) — enables agents to prioritize reactions.
  - **K-bounded staleness guard**: When a shared-mode agent falls behind by >10 events, tool responses include a `[CONTEXT STALE]` warning.
  - **Knowledge conflict detection**: Concurrent writes to the same knowledge key by different agents inject `[CONFLICT]` warnings before proceeding.
  - **Enriched payloads**: Event payloads now include `path`, `category`, `key`, and `reasoning` (from active session task) for richer observability.
  - **SSE backfill on lag**: When a broadcast subscriber falls behind, missed events are automatically backfilled from SQLite instead of dropped.
  - **New REST endpoints**: `GET /v1/context/summary` (materialized workspace view), `GET /v1/events/search` (FTS5 full-text search), `GET /v1/events/lineage` (causal chain traversal).
  - **Team Server scopes expanded**: `ctx_session`, `ctx_knowledge`, `ctx_artifacts`, `ctx_proof`, `ctx_verify` mapped to `sessionMutations`, `knowledge`, `artifacts`, `search` scopes.
  - **Session race fix**: `SharedSessionStore::get_or_load` uses atomic `entry` API to prevent TOCTOU races under concurrent agent loads.
- **Configurable proxy upstreams** — Teams routing through custom API gateways can now set `proxy.anthropic_upstream`, `proxy.openai_upstream`, and `proxy.gemini_upstream` via `lean-ctx config set` or environment variables. Upstreams are resolved once at proxy startup (env > config > default).
- **Proxy upstream diagnostics** — `lean-ctx doctor` validates proxy upstream URLs (self-referential loop detection, URL format) and reports which upstreams are active.
- **6 new adversarial compression tests** — `gh pr diff`, `gh api`, `gh search`, `gh workflow` verbatim passthrough, plus shell-hook-level diff preservation test.

### Changed

- **Dry-run uninstall** — `lean-ctx uninstall --dry-run` now previews Claude Code and Codex proxy cleanup actions.

## [3.5.3] — 2026-05-07

### Fixed

- **Dashboard command counter** — Shell commands in track-only mode (e.g. `git status`, `docker ps`) that use `exec_inherit` are now counted via `exec_inherit_tracked()`, and `record_shell_command` no longer skips zero-token commands. Previously many commands went unrecorded in the dashboard.
- **SLO false positives** — `CompressionRatio` SLO now requires a minimum of 5,000 original tokens before evaluating, and the threshold was raised from 0.75 to 0.90. Eliminates constant "violated CompressionRatio" warnings caused by `full` mode reads.
- **X11 clipboard in vim** — Removed explicit stripping of `DISPLAY`, `XAUTHORITY`, and `WAYLAND_DISPLAY` environment variables from `exec_buffered`, restoring X11 clipboard sync after exiting vim/vi in Claude Code.
- **pack_cmd unwrap** — `LocalRegistry::open()` now returns a graceful error instead of panicking on IO failures.
- **cursor.rs JSON type safety** — `merge_cursor_hooks` now validates JSON types before unwrapping, preventing panics when `hooks.json` contains unexpected structures.

### Added

- **Rules-staleness detection** — On the first MCP tool call of a session, lean-ctx checks whether the agent's rules file contains the current version marker. If outdated, a `[RULES OUTDATED]` warning is injected into the tool response, prompting the agent to re-read rules or run `lean-ctx setup`.

### Changed

- **Codebase maintainability** — Split `doctor.rs` (2,348 lines) into `doctor/{mod,integrations,fix}.rs` and `uninstall.rs` (1,859 lines) into `uninstall/{mod,agents,parsers}.rs` for better modularity.
- **Cloud-server cleanup** — Removed unused `jwt_secret` field from cloud-server config and auth state.

## [3.5.2] — 2026-05-07

### Fixed

- **Agent zombie cleanup** — `cleanup_stale()` now marks dead processes as `Finished` immediately regardless of age, fixing the "phantom agents" bug where terminated MCP sessions (e.g. from Claude Code subagents, `/superpowers`, `/gsd` plugins) stayed listed as "Active" in the Agent World dashboard indefinitely. Previously, agents were only cleaned up after 24 hours. Fixes the issue reported by daviddatu_.
- **Dashboard live-filter** — `build_agents_json()` now calls `cleanup_stale()` on every API request and additionally filters by `is_process_alive()` as a safety net, ensuring the Agent World dashboard never shows zombie entries.
- **CLI/MCP feature parity** — new `core::tool_lifecycle` module ensures CLI commands (`lean-ctx read`, `lean-ctx grep`, `lean-ctx ls`, `lean-ctx -c`) trigger the same side effects as MCP tools: session tracking, Context Ledger updates, heatmap recording, intent detection, and knowledge consolidation. Previously CLI-only users lost ~60% of Context OS features.
- **Daemon double-recording bug** — CLI reads routed through the daemon no longer record a second `(sent, sent)` stats entry with 0% savings, which was diluting the overall savings rate on the dashboard.
- **Search savings accuracy** — `ctx_search` now estimates native grep baseline cost at 2.5× raw match tokens (accounting for context lines, separators, and full paths), up from 1× which showed misleadingly low savings.
- **Track-mode dilution** — Shell commands in track-only mode (no compression) no longer record `(0, 0)` token entries that inflated command counts without contributing savings, improving the dashboard savings rate from ~30% to 86%+.
- **Crash-loop backoff guard** — MCP server startup now detects rapid restart loops (>5 starts in 30s) and applies exponential backoff (up to 60s), preventing system hangs during binary updates.
- **Stats flush for short-lived CLI** — explicit `stats::flush()` calls after CLI `read`, `grep`, `ls`, `diff`, `deps` commands ensure token savings from hook subprocesses are persisted to disk immediately.

### Changed

- **Agent HookMode reclassification** — CRUSH, Hermes, OpenCode, Pi, and Qoder moved from `CliRedirect` to `Hybrid` mode because their hook mechanisms cannot guarantee full interception of all tool types. Only Cursor, Codex CLI, and Gemini CLI remain in pure CLI-redirect mode.
- **Claude Code Hybrid mode** — Claude Code now uses Hybrid mode (MCP + hooks) instead of CLI-redirect. `lean-ctx init --agent claude` installs the MCP server entry in `~/.claude.json` and configures PreToolUse hooks for Bash compression. This ensures full functionality even in headless (`-p`) mode where PreToolUse hooks don't fire.
- **Antigravity dedicated hook** — `lean-ctx init --agent antigravity` now has its own installation function (no longer shares with Gemini CLI), correctly configuring MCP at `~/.gemini/antigravity/mcp_config.json` and hook matchers for Antigravity's native tools (`run_command`, `view_file`, `grep_search`).

## [3.5.1] — 2026-05-06

### Fixed

- **Tool Registry not initialized** — `ctx_tree`, `ctx_discover_tools`, and 23 other trait-based tools returned "Unknown tool" because the registry was never wired up at server startup. All 56 advertised tools are now dispatchable. Fixes #184.
- **Copilot CLI MCP path** — `lean-ctx init --agent copilot` now creates `.github/mcp.json` with the correct `"mcpServers"` key (per GitHub Copilot CLI spec), in addition to `.vscode/mcp.json` with the VS Code `"servers"` key. Previously wrote to the wrong path (`.github/copilot/mcp.json`) with the wrong key format.
- **Agent-scoped project rules** — `lean-ctx init --agent copilot` no longer creates `.cursorrules` or `.claude/rules/` files. Project rules are now scoped to the requested agent(s).
- **SKILL.md for Copilot/VS Code** — `lean-ctx setup` now installs SKILL.md for GitHub Copilot / VS Code users, and `lean-ctx doctor` checks the correct path (`~/.vscode/skills/lean-ctx/SKILL.md`).

## [3.5.0] — 2026-05-06

### Added

- **Context OS Runtime** — full integration of shared sessions, event bus, and SSE endpoints for real-time multi-agent collaboration. Agents can subscribe to context changes, broadcast events, and share session state across workspaces.
- **Daemon Mode** — persistent background daemon with CLI-first dispatch. `lean-ctx daemon start/stop/status` manages the process. All CLI commands route through the daemon for sub-millisecond response times and shared state.
- **Context Package System** — versioned, shareable context bundles with `lean-ctx pack create/list/info/export/import/install/remove/auto-load`. Package layers (knowledge, gotchas, config, graph) enable portable project intelligence.
- **Context Field Theory (CFT)** — unified model for context management with Context Potential Function, Rich Context Ledger, Context Overlay System, Context Handles, and Context Compiler.
- **Provider Framework** — pluggable provider system with GitLab integration and caching layer for external context sources.
- **Autonomy Drivers** — configurable agent autonomy levels with intent routing and degradation policies.
- **Context IR** — intermediate representation for context compilation, enabling cross-provider optimization.
- **Instruction Compiler** — `lean-ctx instructions` command compiles project-specific rules into optimized agent instructions.
- **Context Proof System** — `lean-ctx proof` generates verifiable context provenance chains for audit trails.
- **Team Server: Context OS scopes** — `SessionMutations`, `Knowledge`, and `Audit` scopes for fine-grained team permissions via `lean-ctx team token create`.
- **Qoder & QoderWork support** — new editor integration for Qoder IDE. PR #180 by @zsefvlol.
- **56 MCP tools** — exposed all registered tools for installed agents, including new `ctx_verify`, `ctx_proof`, `ctx_provider`, `ctx_artifacts`, `ctx_index` tools. Fixes #176.
- **38 Context OS integration tests** — comprehensive test suite covering multi-client concurrency, event bus, shared sessions, and SSE endpoints.
- **Windows OpenCode guide** — step-by-step manual for OpenCode on Windows 10. PR #181 by @HamedEmine.

### Changed

- **CLI-First Architecture** — all new modules (daemon, providers, instruction compiler, proof, overview, knowledge, compress, verify) are accessible as CLI subcommands, reducing MCP schema overhead.
- **Server Refactor** — modular tool registry with `ToolTrait`, pipeline stages, and per-tool dispatch for cleaner extensibility.
- **A2A alignment** — `ScratchpadEntry` now aligns with `A2AMessage` types for cross-agent interoperability.
- **HTTP-MCP contract** — extended with full Context OS API surface documentation.
- **Shell pattern library** — expanded to 95+ output compression patterns including clang, fd, glab, just, ninja.
- **Property Graph** — enhanced with metadata layer and reproducibility contract.

### Fixed

- **CLI relative path resolution** — paths are now resolved to absolute before sending to the daemon, preventing "file not found" errors when working directory differs.
- **`install.sh` POSIX compliance** — rewritten as pure POSIX sh so `curl | sh` works on dash (Ubuntu/Debian default). PR #175 by @narthanaj.
- **Qoder MCP config** — added `LEAN_CTX_FULL_TOOLS` to Qoder configuration for complete tool exposure. Includes clippy fixes.
- **Team SSE endpoint** — removed dead code and properly wired `audit_event` into the SSE stream.

## [3.4.7] — 2026-05-01

### Added

- **`ctx_call` meta-tool** — compatibility tool for MCP clients with static tool registries (e.g. Pi Coding Agent). Invoke any `ctx_*` tool by name via a stable schema without requiring dynamic `tools/list` refresh. Fixes #174.
- **Interactive Graph Explorer** — `ctx_graph action=export-html` generates a self-contained, interactive HTML visualization with pan/zoom, node selection, transitive highlighting, and PNG export.
- **Self-Hosted Team Server** — `lean-ctx team serve` enables shared context across workspaces with token-based auth, scoped permissions, rate limiting, and audit logging.

### Changed

- **Dual-format hook output** — `lean-ctx hook rewrite/redirect` now emits a combined JSON response compatible with both Cursor (`permission`/`updated_input`) and Claude Code (`hookSpecificOutput`). All IDEs that support PreToolUse hooks now work with the same command.
- **JetBrains config format** — `~/.jb-mcp.json` now uses the official `mcpServers` snippet format matching JetBrains AI Assistant documentation (was: nonstandard `servers` array).
- **Shell hook block markers** — `lean-ctx init --global` now writes stable `# lean-ctx shell hook — begin/end` markers, making updates idempotent and safe across reinstalls.

### Fixed

- **Claude Code hooks not intercepting subagent calls** — `extract_json_field` in hook handlers was too rigid for pretty-printed or spaced JSON from Claude Code. Now robustly handles all formatting styles. Fixes Discord report.
- **Claude Code hooks overwriting other plugins** — `install_claude_hook_config` now *merges* PreToolUse hooks instead of replacing the entire matcher group, preserving hooks from other plugins (e.g. obra/superpowers).
- **`lean-ctx doctor` false positive "pipe guard missing"** — on Windows Git Bash with XDG config paths, doctor now correctly detects shell hooks in both `~/.lean-ctx/` and `~/.config/lean-ctx/` directories, with both forward and backslash path separators. Fixes Discord report.
- **Pi Coding Agent array parameters** — `get_str_array` now accepts JSON-encoded strings (e.g. `"[\"a\",\"b\"]"`) in addition to native JSON arrays, fixing `ctx_multi_read` for the Pi MCP bridge. Fixes #173.
- **Windows CI test failure** — `workspace_config` tests now use `serde_json::json!` for path serialization, preventing invalid JSON escapes on Windows.

## [3.4.6] — 2026-04-30

### Added

- **Unified call graph tool** — new `ctx_callgraph` supports `direction=callers|callees` behind one stable entry point.
- **Graph diagram in unified graph API** — `ctx_graph` now supports `action=diagram` (with `kind=deps|calls` and optional `depth`).
- **Release-gate hardening tests** — added golden/edge coverage for `tokens.rs`, `preservation.rs`, `handoff_ledger.rs`, and workflow store roundtrips.
- **README entry paths** — new 3-tier onboarding/runtime paths (`Quick`, `Power`, `Enterprise`) with concrete commands and expected outcomes.
- **Knowledge graph auto-bootstrap** — when the dashboard's knowledge graph is empty, lean-ctx now automatically generates initial facts (project root, languages, index stats) so users see data immediately.
- **Startup guard (cross-process lock)** — new `core::startup_guard` module provides file-based locking with stale eviction, used to serialize concurrent startup and background maintenance.
- **Cookbook TypeScript SDK** — real integration examples with typed SDK.

### Changed

- **Deprecation aliases (no breaking change)**:
  - `ctx_callers`/`ctx_callees` now route to `ctx_callgraph` with deprecation hints.
  - `ctx_graph_diagram` now routes to `ctx_graph action=diagram` with deprecation hint.
  - `ctx_wrapped` now routes to `ctx_gain action=wrapped` with deprecation hint.
- **Tool metadata alignment** — descriptors, editor auto-approve lists, and docs updated for the unified entry points and 49-tool manifest.
- **Documentation/version hygiene** — README and VISION now consistently reference 49 MCP tools and current runtime state.
- **Legacy cleanup** — removed unlinked `core/watcher.rs` orphan module (no runtime references).
- **Cloud: OAuth2 client credentials** — cloud sync now supports OAuth2 token-based authentication.
- **Memory: configurable policies + knowledge relations** — knowledge facts support temporal relations and configurable retention policies.

### Fixed

- **SIGABRT under concurrent MCP startup** — multiple agent sessions starting simultaneously could crash the process. Fixed with `catch_unwind` at the process entry point, a cross-process startup lock, and capped Tokio worker/blocking threads. Fixes #171.
- **Dashboard stale index auto-rebuild** — `graph_index` and `vector_index` now detect when indexed files are missing and automatically rebuild, preventing empty Knowledge Graph and broken Compression Lab views.
- **Dashboard Compression Lab path healing** — when a file path from the index no longer exists (e.g. after refactoring), the API now tries suffix/filename matching against indexed files and returns actionable candidates. The UI shows clickable suggestions instead of a bare error.
- **Background maintenance stampede** — rules injection, hook refresh, and version checks are now guarded by a cross-process lock, preventing multiple instances from running expensive maintenance simultaneously during agent session initialization.
- **Panic hardening in verification/stats paths** — replaced remaining production `unwrap()` usage in critical library paths:
  - `core/output_verification.rs` fallback regex paths
  - `core/stats/mod.rs` optional buffer extraction
- **CLI guidance consistency** — `lean-ctx wrapped` now clearly points users to the canonical `lean-ctx gain --wrapped` path.
- **Cookbook npm audit vulnerabilities** — resolved all reported npm audit issues in the cookbook package.

## [3.4.5] — 2026-04-28

### Added

- **Agent Harness: Roles & Permissions** — 5 built-in roles (`coder`, `reviewer`, `debugger`, `ops`, `admin`) with configurable tool policies and shell access. Custom roles via `.lean-ctx/roles/*.toml` with inheritance. Server-side middleware blocks unauthorized tools with clear feedback. `ctx_session action=role` to list/switch roles at runtime.
- **Agent Harness: Budget Tracking** — per-session budget enforcement against role limits (context tokens, shell invocations, cost USD). Warning at 80%, blocking at 100%. `ctx_session action=budget` to check status. Budgets reset on role switch or session reset.
- **Agent Harness: Events** — new `EventKind` variants: `RoleChanged`, `PolicyViolation`, `BudgetWarning`, `BudgetExhausted`. All rendered in TUI Observatory with appropriate icons and colors.
- **Agent Harness: Cost Attribution** — real-time per-tool-call cost estimation using `ModelPricing`, recorded into the budget tracker for accurate USD tracking.
- **Agent Harness documentation** — new docs page with full i18n (53 keys × 11 languages), accessible at `/docs/agent-harness`.
- **`LEAN_CTX_DATA_DIR` for cloud config** — cloud client now respects the `LEAN_CTX_DATA_DIR` environment variable for its config directory. PR #168 by @glemsom.

### Fixed

- **MCP server crash recovery** — tool handler panics no longer kill the server (`panic = "unwind"` + `catch_unwind`). Server returns error message and stays alive for the next call. PR #167 by @DustinReynoldsPE.
- **`lean-ctx setup` ignoring config changes** — running setup a second time no longer silently ignores the user's new choices for `terse_agent` and `output_density`. Values are now upserted instead of skipped when keys already exist in `config.toml`.
- **Dashboard cost mismatch with `lean-ctx gain`** — dashboard computed cost savings with hardcoded pricing ($2.50/M input) while `gain` used dynamic model-specific rates. Dashboard now syncs pricing from the gain API for consistent numbers.
- **`ctx_session` tool description missing actions** — `role` and `budget` actions were implemented but not listed in the MCP tool descriptor, so LLMs couldn't discover them. Now documented in granular tool defs and templates.

### Credits

- @DustinReynoldsPE — MCP panic recovery (PR #167)
- @glemsom — `LEAN_CTX_DATA_DIR` cloud support (PR #168)

## [3.4.4] — 2026-04-28

### Fixed

- **Observatory File Heatmap blank** — the File Heatmap panel in `lean-ctx watch` stayed empty because historical per-file access data was never loaded on TUI startup. Now pre-populates from the persistent `heatmap.json` so file activity is visible immediately. Also fixed `EventTail` offset tracking to prevent event loss during concurrent writes. Fixes #166.
- **Windows agent hook installs** — `dirs::home_dir()` does not respect `HOME`/`USERPROFILE` overrides on Windows, causing hooks to install into incorrect directories during CI and in some user setups. Introduced a centralized `core::home::resolve_home_dir()` that checks `HOME`, `USERPROFILE`, and `HOMEDRIVE+HOMEPATH` before falling back to `dirs::home_dir()`. All 13 agent installers and the hook manager now use this resolver.
- **Windows `claude mcp add-json` invocation** — `.cmd` shims cannot be executed directly via `CreateProcess`; now routes through `cmd /C` for reliable invocation.
- **Clippy 1.95 compliance** — resolved all new lints introduced by Rust 1.95: `needless_raw_string_hashes`, `map_unwrap_or`, `unnecessary_trailing_comma`, `duration_suboptimal_units`, `while_let_loop` across 30+ source files.
- **`cargo-deny` 0.19 migration** — updated `deny.toml` to new schema, removed deprecated advisory fields, added missing dependency licenses (`0BSD`, `CDLA-Permissive-2.0`).
- **Windows benchmark stability** — `bench_rrf_eviction_vs_legacy` no longer panics from `Instant` underflow on short-lived processes.
- **Coverage timeout** — `benchmark_task_conditioned_compression` now skipped under tarpaulin instrumentation and uses smaller input to prevent CI timeouts.
- **Uninstall dry-run** — `lean-ctx uninstall --dry-run` no longer accidentally removes components.

### Changed

- **License updated to Apache-2.0** — all references across the repository and website (11 languages) updated from MIT to Apache-2.0.
- **Clippy pedantic across entire codebase** — comprehensive refactoring to satisfy `clippy::pedantic` with zero warnings: `Copy` derives, `map_or`/`is_ok_and` patterns, `Duration::from_hours/from_mins`, `while let` loops, and raw string simplification.
- **`cfg(tarpaulin)` declared in Cargo.toml** — prevents `unexpected_cfgs` lint failures when coverage attributes are used.

## [3.4.3] — 2026-04-27

### Fixed

- **Pi Agent compression loop** — agents using `pi-lean-ctx` could get stuck in a compression loop where `bash` output was too aggressively compressed, preventing the agent from extracting needed information. The `bash` tool now supports a `raw=true` parameter that bypasses compression entirely when exact output is critical. Fixes #159.
- **Hook handlers ignore `LEAN_CTX_DISABLED`** — `handle_rewrite`, `handle_codex_pretooluse`, `handle_copilot`, and `handle_rewrite_inline` now check `LEAN_CTX_DISABLED` env var and exit immediately when set. This prevents Claude Code subagents and rewind operations from being blocked by hooks. Fixes #162.
- **Telemetry claims in README/SECURITY.md** — replaced inaccurate "Zero telemetry / Zero network requests" claims with honest documentation of what network activity exists (daily version check, opt-in anonymous stats). Fixes #160.

### Added

- **Version check opt-out** — new `update_check_disabled = true` config option and `LEAN_CTX_NO_UPDATE_CHECK=1` env var to completely disable the daily version check against `leanctx.com/version.txt`.
- **Pi Agent `raw` parameter** — `bash` tool in `pi-lean-ctx` now accepts `raw=true` to skip compression, matching `ctx_shell raw=true` behavior in the MCP server.
- **`is_disabled()` guard** — centralized helper in `hook_handlers.rs` for consistent `LEAN_CTX_DISABLED` checks across all hook entry points.
- **New integration tests** — `hook_rewrite_disabled_produces_no_output` and `codex_pretooluse_disabled_exits_cleanly` verify the disabled guard behavior. `run_hook_test` helper explicitly removes inherited env vars to prevent test pollution.

### Changed

- **Data sharing default flipped** — `lean-ctx setup` now asks `[y/N]` (opt-in) instead of `[Y/n]` (opt-out). Users must explicitly choose to enable anonymous stats sharing.
- **Pi Agent tool prompts overhauled** — `description` fields for all 5 Pi tools (`bash`, `read`, `ls`, `find`, `grep`) rewritten to provide clear guidance on which tool to use for which task, aligning with Pi Agent's architecture where `description` is the primary LLM guidance field. Redundant `promptGuidelines` removed from `ls`/`find`/`grep`.
- **Pi Agent explicit entry point** — `pi-lean-ctx` now uses `./extensions/index.ts` as explicit entry point instead of relying on default resolution. PR #158 by @riicodespretty.

### Credits

- @glemsom — Pi Agent prompt improvements (PR #157) and architectural insights on `promptGuidelines` behavior (PR #161)
- @johnwhoyou — `LEAN_CTX_DISABLED` hook handler fix (PR #163)
- @riicodespretty — explicit extension entry point (PR #158)
- @pavelxdd — telemetry transparency request (Issue #160)

## [3.4.2] — 2026-04-26

### Fixed

- **Unicode SIGABRT in `ctx_overview`** — directory path truncation used byte-index slicing (`&dir[len-47..]`) which panicked on multi-byte UTF-8 characters (Chinese, Japanese, Korean, emoji paths). Replaced with `truncate_start_char_boundary()` that respects char boundaries. Fixes #154.
- **Windows shell detection in Git Bash / MSYS2** — `find_real_shell()` now checks `MSYSTEM`/`MINGW_PREFIX` env vars before `PSModulePath`, preventing incorrect PowerShell detection when running inside Git Bash. Fixes #156.

### Added

- **Shell hint in MCP instructions (Windows)** — on Windows, instructions now include detected shell type with explicit guidance (e.g. "SHELL: bash (POSIX). Use POSIX commands, not PowerShell cmdlets"), helping LLMs generate correct commands for the active shell environment.
- **Shell mismatch hint in `ctx_shell` responses (Windows)** — when a command fails and contains PowerShell cmdlets while the detected shell is POSIX, a correction hint is appended to the response.
- **`shell_name()` public API** — returns the short shell basename (e.g. "bash", "powershell", "cmd") for use in instructions and hints.

## [3.4.1] — 2026-04-25

Performance and token optimization release. Reduces per-session overhead by up to 64%.

### Added

- **`LEAN_CTX_NO_CHECKPOINT` env var** — disable auto-checkpoint injection independently from `minimal_overhead`
- **`PreparedSave` pattern** — `Session.save()` split into `prepare_save()` (CPU-only serialization under lock) + `write_to_disk()` (background I/O via `tokio::task::spawn_blocking`), removing disk I/O from the tool response hot path
- **`md5_hex_fast`** — 8x faster fingerprinting for outputs >16 KB by hashing prefix + suffix + length instead of full content
- **Benchmark tests** — 8 new tests covering token overhead budgets, cache effectiveness, compression density, session save latency, and MD5 performance

### Changed

- `count_tokens` called once per tool response (was up to 4x) — cached result reused for hints, cost attribution, and logging
- `CostStore` writes deferred to background thread via `spawn_blocking`
- `mcp-live.json` writes debounced to every 5th tool call (80% fewer disk writes)
- `compress_output` skipped entirely for `Normal` density (no string copy)
- Auto-checkpoint, meta-strings (savings/stale notes, shell hints, archive hints), and session blocks now all suppressed under `minimal_overhead`

### Fixed

- Integer overflow crash in `shell_efficiency_hint` when output tokens exceeded input tokens — now uses `saturating_sub`
- Synchronous `save()` restores retry counter on disk write failure, preserving auto-save semantics

## [3.4.0] — 2026-04-25

Addresses GitHub issues #150, #151, #152, #153.

### Changed (BREAKING)

- **Lazy tools now the default** — Only 9 core tools are exposed by default instead of 46. This reduces per-turn input token overhead by ~80%. Use `LEAN_CTX_FULL_TOOLS=1` to opt back in to all tools. The `ctx_discover_tools` tool lets agents discover and load additional tools on demand. (#153)

### Added

- **JSONC comment support** — `lean-ctx setup` and all editor config writers now parse JSON with `//` and `/* */` comments using a built-in JSONC stripper. Config files with comments (e.g. `opencode.json`) are no longer treated as invalid and overwritten. (#151)
- **XDG Base Directory compliance** — New installs use `$XDG_CONFIG_HOME/lean-ctx` (default `~/.config/lean-ctx/`) instead of `~/.lean-ctx`. Existing `~/.lean-ctx` directories are detected and used automatically — no migration required. (#152)
- **`minimal_overhead` config option** — Set `minimal_overhead = true` in config or `LEAN_CTX_MINIMAL=1` env var to skip session/knowledge/gotcha blocks in MCP instructions, minimizing token overhead for cost-sensitive workflows. (#153)
- **Shell hook disable** — New `--no-shell-hook` flag for `lean-ctx init`, `shell_hook_disabled = true` config option, and `LEAN_CTX_NO_HOOK=1` env var to disable the `_lc()` shell wrapper across all shells (bash, zsh, fish, PowerShell). MCP tools remain fully active. (#150)

### Fixed

- Shell hook source lines now use the resolved data directory path instead of hardcoded `~/.lean-ctx`, matching XDG-compliant installations.
- `upsert_source_line` detection works for both legacy and XDG hook paths (including Windows backslash paths).

## [3.3.9] — 2026-04-24

### Security & Safety Hardening (GitHub Issue #149)

Comprehensive response to the [TheDecipherist adversarial security review](https://github.com/TheDecipherist/rtk-test/blob/main/docs/rtk-findings.md) comparing lean-ctx vs RTK across 16 safety-critical scenarios. The review was conducted against v3.2.5 — many findings were already fixed in 3.3.x, and v3.3.9 addresses the remaining gaps.

#### Already Fixed (confirmed with adversarial tests since v3.3.x)
- **`git diff` code content**: `compress_diff_keep_hunks()` preserves all `+`/`-` changed lines, only trims context to max 3 lines per hunk
- **`df` root filesystem**: Verbatim passthrough — no compression applied to `df` output
- **`pytest` xfail/xpass**: Summary explicitly includes `xfailed`, `xpassed`, `skipped`, and `warnings` counters
- **`git status` DETACHED HEAD**: Passes through verbatim including "HEAD detached at" warning
- **`ls` shows `.env`**: No file filtering — all files including `.env` are shown
- **`pip list` all packages**: Full package list preserved — no truncation
- **`git stash` verbatim**: Passes git stash output through unchanged
- **`ruff` file:line:col**: Preserves all location references in linter output
- **`find` full paths**: Preserves complete absolute paths
- **`wc` via pipe**: Correctly reads stdin (piped input)
- **Log `CRITICAL`/`FATAL` severity**: `log_dedup` and `safety_needles` explicitly recognize and preserve CRITICAL, FATAL, ALERT, EMERGENCY severity levels

#### Fixed in v3.3.9
- **`git show` diff content** (CRITICAL): `compress_show()` now preserves full diff content using `compress_diff_keep_hunks()` instead of reducing to `hash message +N/-M`. Code review via `git show` is now safe.
- **`docker ps` health status** (CRITICAL): Added fallback detection for `(unhealthy)`, `(healthy)`, `(health: starting)`, and `Exited(N)` annotations that survive even when column-based parsing misaligns.
- **`git log` default cap** (HIGH): Increased from 50 to 100 entries (was ~20 in v3.2.5). With explicit `-n`/`--max-count`, no limit is applied. Truncation message clearly indicates omitted count.

#### New Adversarial Tests
- `adversarial_git_show_preserves_diff_content` — verifies code changes survive `git show`
- `adversarial_git_show_preserves_security_change` — verifies security-relevant removals (e.g. CSRF) are visible
- `adversarial_docker_ps_unhealthy_narrow_columns` — verifies health status survives tight column layouts
- `adversarial_docker_ps_exited_containers` — verifies crashed containers are shown
- `adversarial_git_log_100_plus_commits` — verifies 100-entry cap and truncation message
- `adversarial_git_log_explicit_limit_unlimited` — verifies `-n` bypasses default cap
- `adversarial_safeguard_ratio_prevents_over_compression` — verifies safety net prevents >85% compression
- `adversarial_shell_hook_preserves_errors_in_truncation` — verifies CRITICAL/ERROR lines survive shell hook truncation

### Dependency Security
- **rustls-webpki**: Confirmed already on patched version 0.103.13 (GHSA-82j2-j2ch-gfr8, DoS via panic on malformed CRL BIT STRING)

## [3.3.8] — 2026-04-24

### Bug Fixes
- **Windows TOML path quoting** (GitHub Issue #147): `lean-ctx update` and `lean-ctx setup` now write Windows paths in Codex `config.toml` using TOML single-quoted literal strings (`'C:\...'`) instead of double-quoted strings. Double-quoted TOML strings treat backslashes as escape sequences, causing Codex to fail with "too few unicode value digits". Affects all Windows users with backslash paths in Codex MCP config.

### Improvements
- **Leaner `ls` output** (PR #148 by @glemsom): `lean-ctx ls` now runs plain `ls` instead of `ls -la` by default, reducing token overhead. The agent can add `-la` flags when needed.

## [3.3.7] — 2026-04-23

### New Features
- **`lean-ctx ghost` CLI**: New command that reveals hidden token waste — shows unoptimized shell commands, redundant reads, and oversized contexts with monthly USD savings estimate. Supports `--json` for CI integration.
- **`ctx_review` MCP tool**: Automated code review combining impact analysis (`ctx_impact`), caller tracking (`ctx_callers`), and test file discovery. Three actions: `review` (full analysis), `diff-review` (review changed files from git diff), `checklist` (structured review questions).
- **Content-Defined Chunking** (Rabin-Karp): Opt-in rolling-hash chunking for `ctx_read` that creates stable chunk boundaries, improving LLM prompt cache hit rates across edits. Enable via `content_defined_chunking = true` in `config.toml`.
- **Claude Code Plugin Manifest**: `.claude-plugin/manifest.json` added for future Claude Code plugin marketplace integration.

### Improvements
- **Cache-Safety Doctor Check**: `lean-ctx doctor` now verifies that `cache_alignment` and `provider_cache` modules are operational (12 checks total).
- **`provider_cache` module activated**: Previously dormant cache provider module is now wired into the diagnostic pipeline.

## [3.3.6] — 2026-04-23

### Security Hardening
- **GitHub Actions pinned to SHA**: All 10 Actions across CI, Release, and CodeQL workflows are now pinned to immutable commit SHAs instead of mutable version tags, preventing supply-chain attacks. (CodeQL #24-#36)
- **File system race condition fixed**: TOCTOU vulnerability in VS Code extension's MCP config writer eliminated. (CodeQL #37)
- **CodeQL Python false positive resolved**: Stale `language:python` scan configuration removed; explicit CodeQL workflow now covers only Rust, JavaScript/TypeScript, and Actions.
- **Email masking in CLI**: `lean-ctx login/register/forgot-password` now mask email addresses in console output. (CodeQL #21-#23)

### Bug Fixes
- **TypeScript `.js` import resolution** (GitHub Issue #146): The graph builder now correctly resolves relative `.js` specifiers to `.ts` source files per the TypeScript module resolution spec. Covers `.js→.ts/.tsx`, `.jsx→.tsx/.ts`, `.mjs→.mts`, `.cjs→.cts`.
- **Graceful client disconnect**: When an IDE cancels the MCP connection before initialization completes, lean-ctx now exits silently instead of printing a confusing `expect initialized request` error.
- **Session ID uniqueness**: Session IDs now include an atomic counter suffix, preventing collisions when two sessions are created within the same millisecond.

### Improvements
- **Environment variable forwarding** (PR #144 by @glemsom): `pi-lean-ctx` now forwards the parent process environment to the lean-ctx subprocess, so config env vars (`LEAN_CTX_TERSE_AGENT`, `LEAN_CTX_ALLOW_PATH`, etc.) work correctly.

## [3.3.5] — 2026-04-23

### Multi-Project Workspace Support (GitHub Issue #141)
- **`allow_paths` in config.toml**: New config field to explicitly allow additional paths in PathJail. Useful for mono-repos and multi-project workspaces where projects live outside the detected root.
- **Auto-detect multi-root workspaces**: When the CWD has no project markers but contains 2+ child directories with markers (`.git`, `Cargo.toml`, `package.json`, etc.), lean-ctx auto-detects this as a workspace and allows all child projects via PathJail.
- **Improved error messages**: PathJail errors now include a hint suggesting `LEAN_CTX_ALLOW_PATH` or `allow_paths` in `config.toml`.

### Windows PowerShell Fixes (GitHub Issue #142)
- **Pipe-guard in profile snippet**: The `[Console]::IsOutputRedirected` check is now embedded directly in the PowerShell profile source line, preventing errors when IDEs redirect stdout.
- **Binary path resolution**: `resolve_portable_binary()` now takes only the first line of `where` output on Windows, and prefers `.cmd`/`.exe` variants to avoid corrupted path detection.

### CLI Improvements
- **`excluded_commands` via CLI** (PR #143 by @glemsom): `lean-ctx config set excluded_commands "make,go build"` now works.

### CI Stability
- **Fixed flaky test**: `startup_prefers_workspace_scoped_session` race condition resolved with timestamp separation.
- **Windows CI**: Python-dependent sandbox tests now skip gracefully when Python is unavailable on the runner.

## [3.3.4] — 2026-04-23

### Heredoc Support (GitHub Issue #140)
- **Smart heredoc detection in `ctx_shell`**: Heredocs are no longer blanket-rejected. Only heredoc + file redirect combinations (`cat <<EOF > file.txt`) are blocked. Legitimate uses like `psql <<EOF`, `git commit -m "$(cat <<'EOF'...)"`, and input piping are now allowed through.
- **Hook passthrough for heredoc commands**: The PreToolUse hook (Claude Code, Codex, Copilot) no longer wraps heredoc-containing commands in `lean-ctx -c '...'`. Heredocs cannot survive the quoting round-trip (newlines get escaped to `\\n`), so they are passed through to the shell directly.

### Headless MCP Mode
- **New `LEAN_CTX_HEADLESS=1` environment variable**: When set, the MCP server skips all auto-setup during `initialize()` — no rules injection, no hook updates, no version check, no agent registry writes. Session management and all MCP tools remain fully functional. Designed for users who manage their own configuration (e.g. custom launchers with `--append-system-prompt`).

### Cloud Auth Hardening
- **Login and Register are now separate commands**: `lean-ctx login` only calls `/api/auth/login`. `lean-ctx register` only calls `/api/auth/register`. The previous behavior auto-fell back to registration on any non-specific login error (network, 500, DNS), which caused users to unknowingly create duplicate accounts.
- **Clear error messages**: Specific guidance for wrong password, unverified email, non-existent account, and server errors.

### Interactive Setup with Premium Features
- **Setup wizard extended to 7 steps**: New "Premium Features" step offers configuration of Terse Agent Mode (off/lite/full/ultra), Tool Result Archive (on/off), and Output Density (normal/terse/ultra) during `lean-ctx setup`.

### Dependency Updates
- **Dependabot #12 resolved**: `rand 0.8.5` phantom dependency removed via `cargo update` (GHSA-cq8v-f236-94qc).
- Updated: `tokio` 1.52.1, `rustls` 0.23.39, `rmcp` 1.5.0, `uuid` 1.23.1, and 20+ other transitive dependencies.

### Premium Features — Tool Result Archive, Terse Agent Mode, Compaction Survival

#### Tool Result Archive + ctx_expand (Zero-Loss Compression)
- **Archive-on-disk**: Large tool outputs (>4096 chars) are automatically archived to `~/.lean-ctx/archives/` before density compression. The compressed response includes an `[Archived: ... Retrieve: ctx_expand(id="...")]` hint so the agent can retrieve the full original output at any time.
- **New MCP tool `ctx_expand`**: Retrieve archived tool output by ID. Supports full retrieval, line-range retrieval (`start_line`/`end_line`), pattern search (`search`), and listing all archives (`action="list"`).
- **Session-scoped archives**: Each archive entry is tagged with the session ID, enabling per-session listing and cleanup.
- **TTL-based cleanup**: Archives older than `max_age_hours` (default 48h) are automatically cleaned up. Configurable via `archive.max_age_hours` in `config.toml` or `LEAN_CTX_ARCHIVE_TTL` env var.
- **Idempotent storage**: Content-hash-based IDs ensure the same output is never stored twice.
- **Config**: `archive.enabled`, `archive.threshold_chars`, `archive.max_age_hours`, `archive.max_disk_mb` in `config.toml`. Env overrides: `LEAN_CTX_ARCHIVE`, `LEAN_CTX_ARCHIVE_THRESHOLD`, `LEAN_CTX_ARCHIVE_TTL`.

#### Bidirectional Token Optimization (Terse Agent Mode)
- **New `terse_agent` config**: Controls agent output verbosity via instructions injection. Levels: `off` (default), `lite` (concise, bullet points), `full` (max density, diff-only), `ultra` (expert pair-programmer, minimal narration).
- **Smart CRP interaction**: Terse `lite`/`full` are skipped when CRP mode is `tdd` (already maximally dense). `ultra` always applies as an additional layer.
- **CLI toggle**: `lean-ctx terse <off|lite|full|ultra>` for instant switching.
- **Per-project override**: `terse_agent = "full"` in `.lean-ctx.toml`.
- **Env override**: `LEAN_CTX_TERSE_AGENT=full`.

#### Compaction Survival (Session-Resilience)
- **`build_resume_block()`**: Generates a compact (~500 token) session resume containing task, decisions, modified files, next steps, archive references, and stats.
- **Automatic injection**: The resume block is injected into MCP instructions whenever an active session with tool calls exists, ensuring context survives agent compaction events.
- **New `ctx_session(action="resume")` action**: Explicit retrieval of the resume block for agents that need on-demand session state.

### Bug Fixes

#### `ctx_expand` not registered in MCP tool listing
- **Fixed**: `ctx_expand` was implemented (dispatch handler, archive storage, tool definition in `list_all_tool_defs()`) but was missing from `granular_tool_defs()` — the function that the MCP server actually uses to build the `tools/list` response. Agents could never discover or call `ctx_expand` despite the feature being fully coded. Now registered as tool #47.

#### `TerseAgent::effective()` ignores environment variable
- **Fixed**: `TerseAgent::effective()` was supposed to let `LEAN_CTX_TERSE_AGENT` override the config.toml value, but fell through to the config value when the env var was set to `"off"`. Rewritten to explicitly check the env var first, then fall back to config.

#### CLI dispatch sync — `terse`, `register`, `forgot-password` not wired in `main.rs`
- **Fixed**: `lean-ctx terse`, `lean-ctx register`, and `lean-ctx forgot-password` were implemented in `cli/dispatch.rs` but the primary dispatch in `main.rs` was missing the match arms. All three commands now work from the CLI.
- **New**: `lean-ctx forgot-password <email>` — sends a password reset email via the LeanCTX Cloud API. Previously referenced in help text but not implemented.
- **Help text**: Updated in both `main.rs` and `cli/dispatch.rs` to consistently list `terse`, `register`, and `forgot-password`.

#### `lean-ctx doctor` ignores `LEAN_CTX_DATA_DIR` (Discord: GlemSom)
- **Fixed**: `doctor` now uses `lean_ctx_data_dir()` instead of hardcoded `~/.lean-ctx` at all 4 locations: shell-hook checks, Docker env.sh path, data directory check, and `compact_score()`. Users with custom `LEAN_CTX_DATA_DIR` will now see correct paths in doctor output.

#### Windows "path escapes project root" (GitHub Issue #139)
- **Fixed**: `pathjail.rs` now uses `safe_canonicalize_or_self()` (which strips the `\\?\` verbatim prefix) instead of raw `std::fs::canonicalize()`. This resolves the mismatch where Windows canonicalized paths (`\\?\C:\Users\...`) didn't match normal paths (`C:/Users/...`), causing false "path escapes project root" errors on Windows with Codex.
- **Windows path normalization hardened**: `is_under_prefix_windows` now strips `\\?\` prefix before comparison, and `allow_paths_from_env` uses the safe canonicalization consistently.

### Shell Quoting Hardening

#### Bug fixes — Argument preservation for complex shell commands
- **Direct argv execution in `-t` mode**: Shell aliases (`_lc gh`, `_lc find`, etc.) now bypass the argv-to-string-to-argv round-trip entirely when multiple arguments are present. `exec_argv()` calls `Command::new().args()` directly, preserving em-dashes (`—`), `#` signs, nested quotes, and all other special characters exactly as the user's shell parsed them. Single-string commands still use `sh -c` for backward compatibility.
- **Single-quote wrapping for hook rewrites**: `wrap_single_command` in hook handlers now uses POSIX single-quote escaping (`'...'` with `'\''` for embedded single quotes) instead of double-quote escaping. This protects `$`, backticks, `!`, and `"` from unintended expansion when commands are passed through Claude Code, Codex, or Copilot hooks.
- **`gh` added to full passthrough**: All `gh` CLI commands (not just `gh auth`) are now excluded from compression and tracking. The GitHub CLI's output is typically short, and its complex argument patterns (multi-word `--comment` values, issue references with `#`) are prone to quoting issues.

#### Code quality
- 20+ new unit tests covering: `exec_direct` / `exec_argv` direct execution, `quote_posix` edge cases (em-dash, `$`, backtick, nested quotes), `wrap_single_command` special characters (`$HOME`, backticks, `find` with long exclude lists, `!`), and `gh` full passthrough verification.
- All integration tests updated for new single-quote format.

## [3.3.3] — 2026-04-28

### Session Stability + Dashboard Clarity

#### Bug fixes — Session root handling (PR #138)
- **Stale session root across checkouts**: Fixed issue where switching between project directories could load a session from a different workspace. New `load_latest_for_project_root()` scans all session files and returns the most recent session matching the target project root, using canonicalized path comparison.
- **Session normalization extracted**: `normalize_loaded_session()` now handles empty-string cleanup and stale project root healing in a single place, called from both `load_by_id()` and `load_latest_for_project_root()`.
- **Startup context detection**: New `detect_startup_context()` derives the correct project root and shell working directory at MCP server startup, even when the IDE provides only a subdirectory path (e.g. `repo/src`).
- **Trusted re-rooting**: `resolve_path()` now checks `startup_project_root` before allowing session re-rooting from absolute paths. Only paths matching the trusted startup root can trigger a re-root, preventing accidental session takeover by untrusted paths.
- **Helper functions**: Added `session_matches_project_root()`, `has_project_marker()`, and `is_agent_or_temp_dir()` to `session.rs` for robust session matching and stale-root detection.

#### Improvements — Dashboard and metrics clarity
- **0%-savings tools hidden from `lean-ctx gain`**: Write-only tools like `ctx_edit` that don't compress output are no longer shown in the "Top Commands" section, preventing confusing "0% savings" entries.
- **0%-savings tools hidden from `ctx_metrics`**: The MCP `ctx_metrics` tool now filters out tools with zero token activity from the "By Tool" breakdown.

#### Code quality
- Fixed all clippy warnings: resolved `MutexGuard` held across await points in tests, `vec!` macro used where array literal suffices, and `Default::default()` struct update with all fields specified.
- All 1295 tests pass with zero warnings, zero clippy errors, full parallel execution.

#### Closed issues
- **#137** (stale session root across checkouts): Fixed by PR #138.

## [3.3.2] — 2026-04-22

### Codex Hook Fix + Docker Knowledge Collision Prevention

#### Bug fixes — Codex CLI integration (PR #136)
- **Codex PreToolUse hook**: Added dedicated `handle_codex_pretooluse()` handler that uses block-and-reroute pattern (exit code 2) instead of the incompatible `updatedInput` field. Commands matched by lean-ctx compression rules are blocked with an actionable re-run suggestion.
- **Codex SessionStart hook**: New `handle_codex_session_start()` injects a short instruction telling Codex to prefer `lean-ctx -c "<command>"` for shell commands.
- **Refactored rewrite logic**: Extracted `rewrite_candidate()` from `handle_rewrite()` to share rewrite detection across Claude Code, Codex, Copilot, and inline-rewrite handlers. Eliminates duplicated skip/wrap/compound logic.
- **New `hooks/support.rs` module**: Shared helpers for hook installation — `install_named_json_server`, `upsert_lean_ctx_codex_hook_entries`, `ensure_codex_hooks_enabled`. Reduces code duplication across agent integrations.
- **Hook dispatch updated**: `lean-ctx hook codex-pretooluse` and `lean-ctx hook codex-session-start` subcommands added to both `main.rs` and `dispatch.rs`.
- **Doctor integration**: `doctor --fix` now sets `LEAN_CTX_QUIET=1` when running in JSON mode to suppress noisy setup output.

#### Bug fixes — Knowledge hash collisions in Docker environments
- **New `project_hash.rs` module**: Composite project hashing that combines the project root path with a detected project identity marker. Prevents knowledge collisions when different projects share the same Docker mount path (e.g. `/workspace`).
- **8 identity detection sources** (checked in priority order):
  1. `.git/config` → remote "origin" URL (normalized: lowercase, stripped `.git` suffix, SSH→path conversion)
  2. `Cargo.toml` → `[package] name`
  3. `package.json` → `"name"` field
  4. `pyproject.toml` → `[project] name` or `[tool.poetry] name`
  5. `go.mod` → `module` path
  6. `composer.json` → `"name"` field
  7. `settings.gradle` / `settings.gradle.kts` → `rootProject.name`
  8. `*.sln` → solution filename
- **Backward compatible**: When no identity marker is found, hash falls back to path-only (identical to pre-3.3.2 behavior). Existing projects without git/manifest files see zero change.
- **Auto-migration**: On `load()`, if the new composite hash directory doesn't exist but the old path-only hash does, knowledge files are automatically copied to the new location. Ownership verification prevents one project from claiming another's data.
- **Consolidated hashing**: Removed duplicate `hash_project()` from `gotcha_tracker.rs` — now uses shared `project_hash::hash_project_root()`.
- **20 new tests**: Collision avoidance, identity detection for all 8 ecosystems, git URL normalization, migration file copying, ownership verification (accept/reject), backward compatibility, empty directory handling.

#### Closed issues
- **#125** (feat: more cmdline compression): Closed — all requested patterns (bun, deno, vite) already implemented in v3.3.0+ and expanded further in v3.3.1.
- **#135** (bug: Codex PreToolUse hook uses unsupported updatedInput): Fixed by PR #136.

## [3.3.1] — 2026-04-18

### Shell Hook Hardening: Complete Developer Environment Coverage

Addresses user-reported issues where `npm run dev` hangs and shell compression is too aggressive for human-readable output. Massively expands passthrough command coverage across all developer ecosystems.

#### Bug fixes
- **`npm run dev` no longer hangs**: Script runner commands (`npm run dev`, `yarn start`, `pnpm serve`, `bun run watch`, etc.) are now recognized as long-running processes and bypass compression entirely. Previously, `exec_buffered` would wait forever for the dev server to exit.
- **`npm run` compression less aggressive**: `compress_run` now shows up to 15 lines verbatim (was 5) and keeps the last 10 lines of longer output (was 3).
- **Case-sensitive passthrough patterns fixed**: Patterns like `bootRun`, `-S`, `-A`, `-B` now correctly match after case normalization in `is_excluded_command`.

#### Shell passthrough expansion (~85 new entries)
- **Package manager script runners**: `npm run dev/start/serve/watch/preview/storybook`, `npm start`, `npx`, `pnpm run dev/start/serve/watch`, `pnpm dev/start/preview`, `yarn dev/start/serve/watch/preview/storybook`, `bun run dev/start/serve/watch/preview`, `bun start`, `deno task dev/start/serve`, `deno run --watch`
- **Python**: `flask run`, `uvicorn`, `gunicorn`, `hypercorn`, `daphne`, `django-admin runserver`, `manage.py runserver`, `python -m http.server`, `streamlit run`, `gradio`, `celery worker/beat`, `dramatiq`, `rq worker`, `ptw`, `pytest-watch`
- **Ruby/Rails**: `rails server/s`, `puma`, `unicorn`, `thin start`, `foreman start`, `overmind start`, `guard`, `sidekiq`, `resque`
- **PHP/Laravel**: `php artisan serve/queue:work/queue:listen/horizon/tinker`, `php -S`, `sail up`
- **Java/JVM**: `gradlew bootRun/run`, `gradle bootRun`, `mvn spring-boot:run`, `mvn quarkus:dev`, `sbt run/~compile`, `lein run/repl`
- **Go**: `go run`, `air`, `gin`, `realize start`, `reflex`, `gowatch`
- **.NET**: `dotnet run`, `dotnet watch`, `dotnet ef`
- **Elixir**: `mix phx.server`, `iex -S mix`
- **Swift**: `swift run`, `swift package`, `vapor serve`
- **Zig**: `zig build run`
- **Rust**: `cargo run`, `cargo leptos watch`, `bacon`
- **Task runners**: `make dev/serve/watch/run/start`, `just dev/serve/watch/start/run`, `task dev/serve/watch`, `nix develop`, `devenv up`
- **CI/CD**: `docker compose watch`, `skaffold dev`, `tilt up`, `garden dev`, `telepresence`, `act`
- **Networking/monitoring**: `mtr`, `nmap`, `iperf/iperf3`, `ss -l`, `netstat -l`, `lsof -i`, `socat`
- **Load testing**: `ab`, `wrk`, `hey`, `vegeta`, `k6 run`, `artillery run`

#### Smart script-runner detection
- New heuristic: any `npm run`/`pnpm run`/`yarn`/`bun run`/`deno task` command where the script name contains `dev`, `start`, `serve`, `watch`, `preview`, `storybook`, `hot`, `live`, or `hmr` is automatically treated as passthrough. Catches variants like `npm run dev:ssr`, `yarn start:production`, `pnpm run serve:local`, `bun run watch:css`.

#### New adversarial tests (12 tests)
- `npm install` package name/count preservation
- `npm install` explicit package names (`express`, `lodash`, `axios`)
- `cargo build` error codes (E0308, E0599) with file:line
- `eslint` rule IDs and error counts
- `go build` file:line error locations
- `docker build` step failure errors
- `tsc` type error codes (TS2304, TS2339) with file references
- `dotnet build` CS0246 errors and build result
- `composer install` package counts
- `cargo test` failure counts
- `kubectl get pods` CrashLoopBackOff/Error status
- `terraform plan` destructive action preservation

#### New passthrough tests (15 test functions)
Organized by ecosystem: npm, pnpm, yarn, bun/deno, Python, Ruby, PHP, Java, Go, .NET, Elixir, Swift/Zig, Rust, task runners, CI/CD, networking, load testing, smart detection, false-positive guard.

#### Website
- Fixed i18n validation: removed duplicate `docsGettingStarted.evalInit*` keys from 10 locale files that caused GitLab CI pipeline failure.

---

## [3.3.0] — 2026-04-21

### Adversarial Safety Hardening

This release addresses all 7 confirmed DANGEROUS compression findings from the [TheDecipherist/rtk-test](https://github.com/TheDecipherist/rtk-test) adversarial test suite (April 2026). LeanCTX now passes **16/16** comparative safety tests (up from 9/16 in v3.2.5).

#### CRITICAL fixes
- **`git diff` code content preserved**: Compression no longer reduces diffs to `file +N/-M`. All `+`/`-` lines (actual code changes) are preserved. Only `index` headers and excess context lines (>3 per hunk) are trimmed. Large diffs (>500 lines) show first 200 + last 50 lines per file. Security-relevant changes (CSRF bypasses, credential removals) are always visible.
- **`docker ps` health status preserved**: Refactored to header-based column parsing. `(unhealthy)`, `Exited (1)`, and multi-word statuses are always preserved verbatim. Container names and images included in output.
- **`df` verbatim passthrough**: Disk usage output is no longer compressed at all. Root filesystem info (`/dev/sda1 ... /`) can never be hidden by "last N lines" heuristics. Output is typically small (<30 lines), making compression unnecessary.
- **`npm audit` CVE IDs preserved**: Vulnerability details including CVE IDs, severity levels, package names, and fix recommendations are retained (up to 30 detail lines) alongside the summary counts.

#### HIGH fixes
- **`git log` truncation increased to 50**: Default truncation raised from 20 to 50 entries. User-specified `--max-count` / `-n` arguments are now respected (no truncation applied). Truncation message updated to suggest `--max-count=N`.
- **`pytest` xfail/xpass/warnings**: Summary now includes `xfailed`, `xpassed`, and `warnings` counters. Example: `pytest: 15 passed, 1 failed, 2 xfailed, 1 xpassed, 2 warnings (3.5s)`.
- **`grep`/`rg` verbatim up to 100 lines**: Outputs with ≤100 lines pass through unchanged. File grouping and context stripping only applies to larger outputs.
- **`pip uninstall` package names listed**: Shows all successfully uninstalled package names (up to 30) instead of just a count.
- **`docker logs` safety-needle scan**: Middle-section truncation now scans for critical keywords (FATAL, ERROR, CRITICAL, panic, OOMKilled, etc.) and preserves up to 20 safety-relevant lines.

#### Additional hardening
- **`git blame` verbatim up to 100 lines**: Small blame outputs pass through unchanged. Larger outputs summarize by author with line ranges.
- **`curl` JSON sensitive key redaction**: Keys matching `token`, `password`, `secret`, `auth`, `credential`, `api_key`, etc. have their values replaced with `REDACTED` in schema output.
- **`ruff check` file:line:col preserved**: Outputs with ≤30 issues pass through verbatim, preserving all `file:line:col` references. Larger outputs show first 20 references plus rule summary.
- **`log_dedup` regex fix**: Fixed a greedy regex (`[^\]]*` → `[^\]\s]*`) in timestamp stripping that consumed entire log messages, preventing proper deduplication. Added `CRITICAL` to severity detection.
- **`lightweight_cleanup` brace collapse**: Now only activates for outputs >200 lines with runs of >5 consecutive brace-only lines. Inserts `[N brace-only lines collapsed]` marker.
- **Safeguard ratio**: If pattern compression removes >95% of content (on outputs >100 tokens), the original output is returned with a warning to prevent over-compression.

### New: Safety Needles Module

New `safety_needles.rs` module provides centralized safety-critical keyword detection used across all compression paths. Keywords include: `CRITICAL`, `FATAL`, `panic`, `FAILED`, `unhealthy`, `Exited`, `OOMKilled`, `CVE-`, `denied`, `unauthorized`, `error`, `WARNING`, `segfault`, `SIGSEGV`, `SIGKILL`, `out of memory`, `stack overflow`, `permission denied`, `certificate`, `expired`, `corrupt`.

The `truncate_with_safety_scan` function in `shell.rs` ensures these keywords are preserved even during generic middle-section truncation (up to 20 safety-relevant lines kept).

### New: `lean-ctx safety-levels`

New command that displays a transparency table showing exactly how each command type is compressed:

- **VERBATIM** (7 commands): `df`, `git status`, `git stash`, `ls`, `find`, `wc`, `env` — zero compression
- **MINIMAL** (11 commands): `git diff`, `git log`, `docker ps`, `grep`, `ruff`, `npm audit`, `pytest`, etc. — light formatting, all safety-critical data preserved
- **STANDARD** (8 commands): `cargo build`, `npm install`, `eslint`, `tsc`, etc. — structured compression
- **AGGRESSIVE** (4 commands): `kubectl describe`, `aws`, `terraform`, `docker images` — heavy compression for verbose output

Also lists global safety features (needle scan, safeguard ratio, auth detection, min token threshold).

### New: `lean-ctx bypass "command"`

Runs any command with **zero compression** — guaranteed raw passthrough. Sets `LEAN_CTX_RAW=1` internally. Use when you need absolute certainty that output is unmodified:

```bash
lean-ctx bypass "git diff HEAD~1"   # guaranteed unmodified
lean-ctx -c "git diff HEAD~1"      # compressed (hunk-preserving)
```

### New: `lean-ctx init <shell>` (eval pattern)

Shell hook initialization now supports the industry-standard `eval` pattern used by starship, zoxide, atuin, fnm, and fzf. The shell code is always generated by the currently-installed binary, ensuring it's never stale after upgrades:

```bash
# bash: add to ~/.bashrc
eval "$(lean-ctx init bash)"

# zsh: add to ~/.zshrc
eval "$(lean-ctx init zsh)"

# fish: add to ~/.config/fish/config.fish
lean-ctx init fish | source

# powershell: add to $PROFILE
lean-ctx init powershell | Invoke-Expression
```

The existing file-based method (`lean-ctx init --global`) continues to work unchanged.

### New: Adversarial Test Suite in CI

21 dedicated adversarial + regression tests now run on every push/PR via a new `adversarial` job in GitHub Actions CI. Tests cover all 16 comparative scenarios from the external audit plus additional safety regression checks. This ensures compression safety is continuously verified.

### Changed
- `compression_safety.rs`: New module with structured `CommandSafety` table and `SafetyLevel` enum
- `shell_init.rs`: Refactored hook generation into `generate_hook_posix()`, `generate_hook_fish()`, `generate_hook_powershell()` for reuse by both file-based and eval-based init
- `ci.yml`: New `adversarial` job running `cargo test --test adversarial_compression`

## [3.2.9] — 2026-04-20

### Fixed
- **UTF-8 text corrupted on Windows PowerShell** (#131): `lean-ctx -c` with non-ASCII output (Russian, Japanese, Chinese, Arabic, etc.) produced mojibake because `String::from_utf8_lossy` misinterpreted Windows system codepage bytes as UTF-8. Introduced `decode_output()` that tries UTF-8 first, then falls back to Win32 `MultiByteToWideChar` for proper codepage-to-Unicode conversion. On PowerShell, additionally injects `[Console]::OutputEncoding = UTF8` and sets `SetConsoleOutputCP(65001)`. Fixed across shell hook, MCP server execute, and sandbox runners.
- **MCP `ctx_shell` commands hang on stdin** (#132, credit: @xsploit): Child processes spawned by the MCP server inherited the JSON-RPC stdin pipe, causing commands like `git` to block instead of receiving EOF. Fixed by setting `stdin(Stdio::null())` on all MCP child processes. Added `GIT_TERMINAL_PROMPT=0` and `GIT_PAGER=cat` to prevent interactive prompts.

### Added
- **MCP command timeout**: Shell commands executed via `ctx_shell` now have a configurable timeout (default 120s). Override with `LEAN_CTX_SHELL_TIMEOUT_MS` env var. Timed-out commands return exit code 124 with a clear error message.
- **Regression tests**: Added `execute_command_closes_stdin` and `git_version_returns_when_git_is_available` tests to prevent future stdin inheritance regressions.

## [3.2.8] — 2026-04-20

### Fixed
- **Codex `config.toml` parse error** (empty `[]` section header): Uninstall left orphaned `[mcp_servers.lean-ctx.tools.*]` sub-sections when removing the main `[mcp_servers.lean-ctx]` section, producing an invalid empty `[]` header on re-setup. Uninstall now removes all `mcp_servers.lean-ctx.*` sub-sections, and the writer defensively skips `[]` lines.
- **Gemini CLI MCP server not loading** (wrong config path): Setup wrote to `~/.gemini/settings/mcp.json` but Gemini CLI reads MCP servers from `~/.gemini/settings.json` under the `mcpServers` key. The MCP config was never loaded by Gemini CLI. Fixed with a new `GeminiSettings` writer that merges `mcpServers` into the existing `settings.json` without overwriting other keys (e.g. `hooks`).
- **Gemini CLI `autoApprove` not recognized**: Gemini CLI uses `"trust": true` for auto-approval, not `autoApprove`. Fixed to use the correct field.
- **Codex `codex_hooks=false` after reinstall**: Uninstall set `codex_hooks = false` but setup didn't reset it to `true`, leaving hooks disabled.

### Added
- **Autonomous intent inference**: `ctx_read` automatically infers a `StructuredIntent` from file access patterns (after 2+ files touched) without requiring explicit agent calls. `ctx_preload` auto-sets intent from task description when none is active or confidence is low.
- **Auto agent registration**: MCP `initialize` handler automatically registers the connecting agent in the `AgentRegistry` based on client name (Cursor/Claude → coder, Antigravity/Gemini → explorer, etc.). Override via `LEAN_CTX_AGENT_ROLE` env var.
- **Context Layer dashboard tab**: New "Context Layer" tab in the localhost dashboard with Pipeline Stats, Context Window pressure, Mode Distribution, and Context Ledger table. Backed by new API endpoints `/api/pipeline-stats`, `/api/context-ledger`, `/api/intent`.
- **Pipeline & Ledger persistence**: `PipelineStats` and `ContextLedger` now persist to disk (`pipeline_stats.json`, `context_ledger.json`) so dashboard data survives server restarts.
- **Codex/Cursor hooks in setup**: `lean-ctx setup` now explicitly installs Codex hook scripts and Cursor hooks as a dedicated step, ensuring hooks are present even on first setup.

### Changed
- **IDE config audit**: All 13 supported IDE configurations verified against official vendor documentation (Cursor, Claude Code, Codex, Windsurf, VS Code/Copilot, Gemini CLI, Antigravity, Amazon Q, Hermes, Cline, Roo Code, Amp, Kiro).

## [3.2.6] — 2026-04-19

### Fixed
- **Project root stuck at agent sandbox path** (#124): The MCP session could retain a stale project root from a temporary directory (e.g. `~/.claude`, `/tmp/`). Fixed with multi-layer healing: `initialize` now validates roots against project markers, `session::load_by_id` detects and corrects agent/temp roots, and `resolve_path` can auto-update a suspicious root when given an absolute project path. Agents like Codex that start in sandbox directories now correctly resolve the actual project.
- **`lean-ctx gain` showing 0% for Shell Hooks** (#126): Small savings percentages were rounded to 0% in the "Savings by Source" and "Live Observatory" sections. Introduced `format_pct_1dp` for one-decimal-place display, `<0.1%` for very small values, and `n/a` when no input data exists.
- **`install.sh` fails on WSL2/Ubuntu** (`set: Illegal option -o pipefail`): `curl -fsSL leanctx.com/install.sh | sh` failed because `install.sh` used Bashisms but was executed by POSIX `sh` (dash). Added a POSIX-compliant preamble that auto-detects and re-executes under `bash`, with a clear error message if `bash` is unavailable. Both `| sh` and `| bash` now work.
- **Dashboard "Live Observatory" showing 0 tokens saved**: The Live Observatory pulled data exclusively from the active MCP session, ignoring shell hook savings. Now falls back to today's aggregate daily stats when no MCP session is active.

### Added
- **`rules_scope` configuration**: Control where agent rule files are installed — `"global"` (home directory only), `"project"` (repo-local only), or `"both"` (default). Avoids duplicate rule files that waste context tokens. Configurable via `config.toml`, `LEAN_CTX_RULES_SCOPE` env var, `lean-ctx config set rules_scope`, or per-project `.lean-ctx.toml` override.
- **Codex/Claude path jail auto-allowlist**: When running inside Codex CLI (`CODEX_CLI_SESSION` set), `~/.codex` is automatically added to allowed paths. Similarly, `~/.claude` is auto-allowed for Claude Code sessions. No manual `LCTX_ALLOW_PATH` needed.
- **`bunx` and `vp`/`vite-plus` CLI compression** (#125): Shell hook now routes `bunx` commands through the bun compressor and `vp`/`vite-plus` through the Next.js build compressor.
- **`lean-ctx update` auto-refreshes setup**: Running `lean-ctx update` now automatically re-runs the full setup (shell hooks, MCP configs, rules) after updating, even when already on the latest version. Ensures all wiring stays current.
- **Website docs**: `rules_scope` documented on configuration page in all 11 languages.

## [3.2.5] — 2026-04-18

### Fixed
- **Critical: shell hook recursion causing 100% CPU/memory** — The `.zshenv` / `.bashenv` shell hooks introduced in v3.2.4 were missing the `LEAN_CTX_ACTIVE` recursion guard. When an AI agent (Claude Code, Codex, etc.) ran a command, `lean-ctx -c` spawned a new shell that re-triggered the hook infinitely, causing a fork bomb. Fixed by checking `LEAN_CTX_ACTIVE` before intercepting and adding a double-guard in `exec()`. Users must run `lean-ctx setup` after updating to refresh the hooks.

## [3.2.4] — 2026-04-18

### Fixed
- **Git stash compression too aggressive** (#114): `git stash list` with ≤5 entries is now preserved verbatim. `git stash show -p` correctly routes to the diff compressor instead of the stash compressor. Added `safeguard_ratio` to `ctx_shell` to prevent over-compression (minimum 15% of original output preserved).
- **Windows Bash hook path stripping** (#113): On Windows with Git Bash / MSYS2, the lean-ctx binary path had slashes stripped (`E:packageslean-ctx.exe` instead of `/e/packages/lean-ctx.exe`). `resolve_binary()` now applies `to_bash_compatible_path` on all platforms.
- **Windows UNC path breakage** (`\\?\` prefix): `std::fs::canonicalize()` on Windows adds extended-length path prefixes that break tools and string comparisons. New `core::pathutil` module provides `safe_canonicalize()` and `strip_verbatim()` used consistently across graph indexing, session state, path jailing, architecture tool, and hook handlers.
- **Dashboard showing empty graphs**: `detect_project_root_for_dashboard()` was using the MCP session's temp sandbox directory instead of the actual project. Now validates project roots against `.git` and project markers before using them; falls through to `shell_cwd` when project_root is invalid. Added `--project=` CLI flag and `LEAN_CTX_DASHBOARD_PROJECT` env var for explicit override.
- **Dashboard Call Graph/Route Map empty states**: Enriched `/api/call-graph` and `/api/routes` responses with metadata (indexed file count, symbol count, route candidates) so the UI shows actionable guidance instead of generic "nothing found" messages.
- **Codex uninstall incomplete** (#116): `lean-ctx uninstall` now correctly removes the `[mcp_servers.lean-ctx]` section from Codex's TOML config, removes `~/.codex/hooks.json`, and resets the `codex_hooks` feature flag.
- **Repo-local config missing fields** (#98): `merge_local()` now supports `auto_consolidate`, `dedup_threshold`, `consolidate_every_calls`, `consolidate_cooldown_secs`, and bidirectional `silent_preload` override from `.lean-ctx.toml`.

### Added
- **Hermes Agent support** (#112): Full integration for Hermes Agent (Nous Research). `lean-ctx init --agent hermes --global` configures MCP via YAML (`~/.hermes/config.yaml`), creates `HERMES.md` rules. Setup auto-detects `~/.hermes/`, doctor checks Hermes config, uninstall cleans up YAML + rules.
- **Kotlin graph analysis** (#96): `ctx_graph`, `ctx_callers`, and `ctx_callees` now produce meaningful results for Kotlin projects. Tree-sitter-backed import extraction, call-site analysis, type-definition extraction, and Java interop with stdlib filtering.
- **Repo-local configuration** (#98): `.lean-ctx.toml` in project root for per-project overrides. Supports `extra_ignore_patterns` (graph/overview exclusions), autonomy settings, and all config fields. `lean-ctx cache reset --project` clears only current project's cache.
- **Post-update MCP refresh**: `lean-ctx update` now verifies and refreshes MCP configurations for all detected editors after binary replacement.
- **Dashboard "Savings by Source"**: Live Observatory and `lean-ctx gain` now show a breakdown of MCP Tools vs. Shell Hooks with individual compression rates and proportional bars.
- **Pi MCP bridge resilience**: Host-cancelled tool calls are handled cleanly with abort signal forwarding and error normalization. Hung MCP calls timeout after 120s with automatic reconnect and retry for read-safe tools. Bridge status includes diagnostics (last error, hung tool, retry state).

### Community
- Merged PR #111 — fix Windows graph path compatibility (@Chokitus)
- Merged PR #115 — handle host-cancelled MCP tool calls in Pi bridge (@frpboy)
- Merged PR #118 — improve dashboard empty-state UX for Route Map and Call Graph (@frpboy)
- Merged PR #122 — timeout and retry hung MCP tool calls in Pi bridge (@frpboy)

## [3.2.3] — 2026-04-17

### Fixed
- **Claude Code project rules missing** (cowwoc): `lean-ctx init --agent claude-code` now creates `.claude/rules/lean-ctx.md` in the project root (project-local rules), in addition to the existing global `~/.claude/rules/lean-ctx.md`. Claude Code reads both locations.
- **`--help` missing commands** (#109): `watch` (live TUI dashboard) and `cache` (file cache management) were implemented but not listed in `lean-ctx --help`.
- **install.sh fails without Rust** (#108): `curl -fsSL https://leanctx.com/install.sh | sh` now auto-detects missing `cargo` and downloads a pre-built binary instead of failing. Users with Rust still get a source build by default.

## [3.2.2] — 2026-04-17

### Added
- **Smart Shell Mode**: New `-t` / `--track` subcommand for human shell usage — full output preserved, only stats recorded. Shell aliases (`_lc`) now default to track mode instead of compress mode, eliminating unwanted output compression for interactive users.
- **`lean-ctx-mode` shell function**: Switch between `track` (default), `compress`, and `off` modes without editing config files. Available in both POSIX (bash/zsh) and Fish shells.
- **`_lc_compress` shell function**: Explicit compression wrapper for power users who want compressed output in their terminal.
- **Unified Rewrite Registry** (`rewrite_registry.rs`): Single source of truth for all 24+ rewritable commands, used consistently across shell aliases, hook rewrite, and compound command lexer.
- **Compound Command Lexer** (`compound_lexer.rs`): Intelligent splitting of `&&`, `;`, `||` compound commands for selective rewriting — only rewritable segments get wrapped with `-c`.
- **Extended hook support**: Copilot hooks now recognize `runInTerminal`, `run_in_terminal`, `shell`, and `terminal` tool names in addition to `Bash`/`bash`.
- **Dashboard API routes**: New `/api/symbols`, `/api/call-graph`, `/api/routes`, `/api/search` endpoints for the web dashboard.
- **22 IDE/agent targets**: Rules injection now supports Crush, Verdent, Pi Coding Agent, AWS Kiro, Antigravity, Qwen Code, Trae, Amazon Q Developer, and JetBrains IDEs (22 total).

### Fixed
- **Shell commands compressed for humans** (#101): `ls`, `git status`, and other aliased commands were always compressed because `_lc` used `-c`. Now defaults to `-t` (track) which preserves full output.
- **"Authorization required" on Ubuntu** (#101): `exec_buffered` pipe redirection triggered X11/Wayland auth errors on headless Linux. Track mode uses `exec_inherit_tracked` (direct stdio), avoiding this entirely.
- **Token counting accuracy**: `stats::record` now uses `count_tokens()` (tiktoken) instead of byte length for output measurement.
- **Dashboard Windows path normalization**: Compression Lab demo paths now correctly handle Windows absolute paths (merged PR #102).
- **Dashboard "d streak" label**: Fixed to display "days streak" (merged PR #106).

### Community
- Merged PR #102 — fix compression lab path resolution (@frpboy)
- Merged PR #103 — add symbols API route (@frpboy)
- Merged PR #104 — add call graph API route (@frpboy)
- Merged PR #106 — fix dashboard streak label (@frpboy)

## [3.2.1] — 2026-04-17

### Fixed
- **crates.io publish**: Claude Agent Skill assets (`SKILL.md`, `install.sh`) are now packaged inside the Rust crate so `cargo publish` verification succeeds.
- **Release CI**: Build `aarch64-unknown-linux-musl` via `cargo-zigbuild` for reliable ARM64 musl cross-compilation (fixes glibc symbol leaks from `gcc-aarch64-linux-gnu`).

## [3.2.0] — 2026-04-17

### Breaking
- **License changed from MIT to Apache-2.0**. All code from this release onwards is Apache-2.0. Previous releases remain MIT-licensed. See `LICENSE-MIT` for the original license and `NOTICE` for attribution.

### Added
- **Context Engine + HTTP server mode**: `lean-ctx serve` exposes all 48 MCP tools via REST endpoints with rate limiting, timeouts, and graceful shutdown — enables embedding lean-ctx as a library.
- **Memory Runtime (autopilot)**: Adaptive forgetting, salience tagging, consolidation engine, prospective memory triggers, and dual-process retrieval router — all token-budgeted and zero-config.
- **Reciprocal Rank Fusion (RRF) cache eviction**: Replaces the Boltzmann-weighted eviction scoring. RRF handles signal incomparability (recency vs frequency vs size) without tuned weights (K=60).
- **Claude Code 2048-char truncation fix**: Auto-detects Claude Code and delivers ultra-compact instructions (<2048 chars). Full instructions installed as `~/.claude/rules/lean-ctx.md`.
- **Claude Agent Skills auto-install**: `lean-ctx init --agent claude` installs `SKILL.md` + `scripts/install.sh` under `~/.claude/skills/lean-ctx/`.
- **ARM64 Linux support**: `aarch64-unknown-linux-musl` binary in release pipeline. Docker instructions updated for Graviton/ARM64.
- **IDE extensions**: JetBrains (Kotlin/Gradle), Neovim (Lua), Sublime Text (Python), Emacs (Elisp) — all thin-client architecture.
- **Security layer**: PathJail (FD-based, single choke point for 42 tools), bounded shell capture, size caps, TOCTOU prevention in `ctx_edit`, symlink leak fix in `ctx_search`, prompt-injection fencing.
- **Unified Gain Engine**: `GainScore` (0–100), `ModelPricing` (embedded cost table), `TaskClassifier` (13 categories), `ctx_gain` MCP tool, TUI/Dashboard/CLI integration.
- **Docker/Claude Code MCP self-healing**: `env.sh` re-injects MCP config when Claude overwrites `~/.claude.json`. Doctor detects and hints fix.
- **Compression deep optimization**: Thompson Sampling bandits for adaptive thresholds, Tree-sitter AST pruning, IDF-weighted deduplication, Information-Bottleneck task filtering, Verbatim Compaction.
- **`lean-ctx -c` now compresses on TTY** (fixes #100): Previously skipped compression when stdout was a terminal, showing 0% savings.
- **Quality column in `ctx_benchmark`**: Shows per-strategy preservation score (AST + identifier + line preservation).

### Fixed
- **CLI `-c` TTY bypass** (#100): `lean-ctx -c 'git status'` now compresses even in terminal (sets `LEAN_CTX_COMPRESS=1`).
- **Windows `Instant` overflow**: RRF eviction test used `now - Duration` which underflows on Windows. Fixed with `sleep`-based offsets + `checked_duration_since`.
- **rustls-webpki CVE**: Updated from 0.103.11 to 0.103.12 (wildcard/URI certificate name constraint fix).
- **MCP server hangs on large projects**: Parallelized tool calls prevent blocking.
- **Dashboard ERR_EMPTY_RESPONSE in Docker**: Bind host + panic recovery → HTTP 500 JSON instead of empty response.
- **Kotlin graph analysis**: AST-span-based symbol ranges for accurate call-graph edges.

### Refactored
- **Dead code elimination**: Removed 598 lines (unused `eval.rs`, CEP benchmark, dead CLI helpers). Reduced `#[allow(dead_code)]` from 32 to 5.
- **Cache store zero-copy**: Replaced `CacheEntry` clone with lightweight `StoreResult` struct (no content duplication).
- **Entropy dedup**: Precomputed n-gram sets with size-ratio filter (exact Jaccard, no allocation storms).
- **Clippy clean**: 0 warnings with `-D warnings` across all targets (1029 tests passing).

### Community
- Merged PR #94 (responsive dashboard — @frpboy)
- Merged PR #95 (MCP performance — @frpboy)
- Merged PR #97 (Kotlin graph support — @Chokitus)

## [3.1.5] — 2026-04-15

### Fixed
- **`claude_config_json_path()` simplified**: Removed over-complex `parent()` fallback logic that guessed at `.claude.json` locations. Now directly uses `$CLAUDE_CONFIG_DIR/.claude.json` as documented by Claude Code.
- **`lean-ctx init --agent claude` now prints config path**: Previously gave zero feedback about where MCP config was written. Now shows `✓ Claude Code: MCP config created at /path/to/.claude.json` — immediately reveals path mismatches (e.g. Docker USER mismatch writing to `/root/.claude.json` instead of `/home/node/.claude.json`).
- **`refresh_installed_hooks()` hardcoded `~/.claude/`**: Hook detection in `hooks.rs` ignored `$CLAUDE_CONFIG_DIR`, always checking `~/.claude/hooks/` and `~/.claude/settings.json`. Now uses `claude_config_dir()`.
- **Rules injection hardcoded `~/.claude/CLAUDE.md`**: `rules_inject.rs` always wrote to `~/.claude/CLAUDE.md` regardless of `$CLAUDE_CONFIG_DIR`. Now uses `claude_config_dir()`.
- **Uninstall hardcoded `~/.claude/`**: `remove_rules_files()` and `remove_hook_files()` couldn't find Claude Code files when `$CLAUDE_CONFIG_DIR` was set. Now uses `claude_config_dir()`.
- **Doctor display hardcoded `~/.claude.json`**: `lean-ctx doctor` always showed `~/.claude.json` even when `$CLAUDE_CONFIG_DIR` pointed elsewhere. Now shows the actual resolved path.

## [3.1.4] — 2026-04-15

### Added
- **`CLAUDE_CONFIG_DIR` support**: `lean-ctx init --agent claude`, `lean-ctx doctor`, `lean-ctx uninstall`, hook installation, and all Claude Code detection paths now respect the `$CLAUDE_CONFIG_DIR` environment variable. Previously hardcoded to `~/.claude.json` and `~/.claude/`.
- **`CLAUDE_ENV_FILE` Docker hint**: `lean-ctx init --global` and `lean-ctx doctor` now recommend setting `ENV CLAUDE_ENV_FILE` alongside `ENV BASH_ENV` in Docker containers. Claude Code sources `CLAUDE_ENV_FILE` before every command — this is the [officially recommended](https://code.claude.com/docs/en/env-vars) shell environment mechanism.
- **Doctor check for `CLAUDE_ENV_FILE`**: In Docker environments, `lean-ctx doctor` now shows separate checks for both `BASH_ENV` and `CLAUDE_ENV_FILE`.

### Fixed
- **Claude Code `_lc` not found in Docker** (#89): Root cause was that `BASH_ENV` alone doesn't work for Claude Code — it uses `CLAUDE_ENV_FILE` to source shell hooks before each command. Recommended Dockerfile now includes `ENV CLAUDE_ENV_FILE="/root/.lean-ctx/env.sh"`.
- **`CLAUDE_CONFIG_DIR` ignored everywhere**: `setup.rs`, `rules_inject.rs`, `doctor.rs`, `hooks.rs`, `uninstall.rs`, and `report.rs` all hardcoded `~/.claude.json` / `~/.claude/`. Now all paths go through `claude_config_json_path()` / `claude_config_dir()` which check `$CLAUDE_CONFIG_DIR` first.
## [3.1.3] — 2026-04-15

### Docker & Container Support

- **Auto-detect Docker/container environments** via `/.dockerenv`, `/proc/1/cgroup`, and `/proc/self/mountinfo`
- **Write `~/.lean-ctx/env.sh`** during `lean-ctx init --global` — a standalone shell hook file without the non-interactive guard (`[ -z "$PS1" ] && return`) that most `~/.bashrc` files have
- **Docker BASH_ENV warning**: when Docker is detected and `BASH_ENV` is not set, `lean-ctx init` now prints the exact Dockerfile line needed: `ENV BASH_ENV="/root/.lean-ctx/env.sh"`
- **`lean-ctx setup` auto-fallback**: detects non-interactive terminals (no TTY on stdin) and automatically runs in `--non-interactive --yes` mode instead of hanging
- **`lean-ctx doctor` Docker check**: new diagnostic that warns when running in a container with bash but without `BASH_ENV` set

### Critical Fix

- **`BASH_ENV="/root/.bashrc"` never worked in Docker** — Ubuntu/Debian `.bashrc` has `[ -z "$PS1" ] && return` which skips the entire file in non-interactive shells. The new `env.sh` approach bypasses this completely.

## [3.1.2] — 2026-04-14

### Fix Agent Search Loops in Large Projects

#### Fixed

- **Agents looping endlessly on search in large/monorepo projects** — root cause was a triple failure: over-aggressive compression hid search results from the agent (only 5 matches/file, 80-char truncation, then generic_compress cut to 6 lines), loop detection only caught exact-duplicate calls (threshold 12 was far too high), and no cross-tool or pattern-similarity tracking existed. Agents alternating between `ctx_search`, `ctx_shell rg`, and `ctx_semantic_search` with slight query variations were never detected as looping.

#### Improved

- **Smarter loop detection** — thresholds lowered from 3/8/12 to 2/4/6 (warn/reduce/block). Added cross-tool search-group tracking: any 10+ search calls within 300s triggers block regardless of tool or arguments. Added pattern-similarity detection: searching for "compress", "compression", "compress_output" etc. now counts as the same semantic loop via alpha-root extraction.
- **Configurable loop thresholds** — new `[loop_detection]` section in `config.toml` with `normal_threshold`, `reduced_threshold`, `blocked_threshold`, `window_secs`, and `search_group_limit` fields.
- **Better search result fidelity** — grep compression now shows 10 matches per file (was 5) with 160-char line truncation (was 80), preserving full function signatures. `generic_compress` scales with output size (shows ~1/3 of lines, max 30) instead of a fixed 6-line truncation.
- **Search commands bypass generic compression** — grep, rg, find, fd, ag, and ack output is no longer crushed by `generic_compress`. Pattern-specific compression is applied when available, otherwise results are returned uncompressed.
- **Actionable loop-detected messages** — blocked messages now guide agents to use `ctx_tree` for orientation, narrow with `path` parameter, and use `ctx_read mode='map'` instead of generic "change your approach" text.
- **Monorepo scope hints** — when `ctx_search` results span more than 3 top-level directories, a hint is appended suggesting the agent use the `path` parameter to scope to a specific service.

## [3.1.1] — 2026-04-14

### Windows Shell Hook Fix + Security

#### Fixed

- **PowerShell npm/pnpm/yarn broken on Windows** — the `foreach` loop in the PowerShell hook resolved npm to its full application path (`C:\Program Files\nodejs\npm.cmd`). When this path contained spaces, POSIX-style quoting caused PowerShell to output a string literal instead of executing the command. Now uses bare command names, consistent with git/cargo/etc. (fixes [#38](https://github.com/yvgude/lean-ctx/issues/38))
- **PowerShell `_lc` off-by-one** — `$args[1..($args.Length)]` produced an extra `$null` element. Replaced with `& @args` splatting which correctly handles all argument counts.
- **Password shown in cleartext during `lean-ctx login`** — interactive password prompt now uses `rpassword` to disable terminal echo, so passwords are never visible.

#### Improved

- **Shell-aware command quoting** — `shell_join` moved from `main.rs` to `shell.rs` with runtime shell detection. Three quoting strategies: PowerShell (`& 'path'` with `''` escaping), cmd.exe (`"path"` with `\"` escaping), and POSIX (`'path'` with `'\''` escaping). Previously used compile-time `cfg!(windows)` which was untestable and broke Git Bash on Windows.
- **11 new unit tests** for `join_command_for` covering all three shell quoting strategies with paths containing spaces, special characters, and empty arguments.

#### Dependencies

- Added `rpassword 7.4.0` for secure password input.

## [3.1.0] — 2026-04-14

### LeanCTX Cloud — Web Dashboard & Full Data Sync

#### Added — Cloud Dashboard

- **Web Observatory** — full-featured cloud dashboard at `leanctx.com/dashboard` mirroring the local Observatory. Includes Overview, Daily Stats, Commands, Performance (CEP), Knowledge, Gotchas, Adaptive Models, Buddy, and Settings views.
- **Login & Registration** — email/password authentication with email verification, password reset via magic link, and dedicated login/register pages.
- **SPA Navigation** — client-side routing with `history.pushState` for each dashboard view with dedicated URLs (`/dashboard/stats`, `/dashboard/knowledge`, etc.).
- **Timeframe Filters** — 7d/30d/90d/All time filters on Overview and Stats pages with live chart updates.
- **Knowledge Table** — searchable, filterable knowledge entries with category badges, confidence stars, and proper table layout with horizontal scroll on mobile.

#### Added — Complete Data Sync

- **Buddy Sync** — full `BuddyState` (ASCII art, animation frames, RPG stats, rarity, mood, speech) synced as JSON to the cloud and rendered with live animation on the dashboard.
- **Feedback Thresholds Sync** — learned compression thresholds per language synced to the cloud via new `/api/sync/feedback` endpoint and displayed on the Performance page.
- **Gotchas Sync** — both universal and per-project gotchas (`~/.lean-ctx/knowledge/*/gotchas.json`) are merged and synced.
- **CEP Cache Metrics** — daily `cache_hits` and `cache_misses` derived from CEP session data for accurate historical stats (previously hardcoded to 0).
- **Command Stats** — per-command token savings with source type (MCP/Hook) breakdown.

#### Added — Cloud Server

- **REST API** — Axum-based API server with endpoints for stats, commands, CEP scores, knowledge, gotchas, buddy state, feedback thresholds, and adaptive models.
- **PostgreSQL Schema** — tables for users, api_keys, email_verifications, password_resets, stats_daily, knowledge_entries, command_stats, cep_scores, gotchas, buddy_state, feedback_thresholds.
- **Email Verification** — SHA256-token-based email verification flow with configurable SMTP.
- **Password Reset** — secure token-based password reset with expiry.

#### Improved

- **Cost Model alignment** — cloud dashboard now uses the same `computeCost()` formula as the local dashboard (input $2.50/M + estimated output $10/M with 450→120 tokens/call reduction), replacing the previous input-only calculation.
- **Adaptive Models explanation** — expanded Models page with "What Adaptive Models Do For You" (before/after comparison), "How Models Are Built" (4-step flow), and "Compression Modes" reference table.
- **Daily Stats accuracy** — hit rate and cache data now correctly display from CEP-enriched daily stats.
- **Dashboard icons** — all SVG icons render with correct dimensions via explicit CSS utility classes.
- **Stats bar color** — Original tokens bar changed to blue for better visibility against the green Saved bar.

#### Removed

- **Teams & Leaderboard** — removed team creation, invites, and leaderboard features in favor of utility-focused dashboard.
- **File Watcher** — removed unused `watcher.rs` module.

#### Security

- **rand crate** — updated to `>= 0.9.3` to fix unsoundness with custom loggers (GHSA low severity).

#### Fixed

- **Token count test threshold** — updated `bench_system_instructions_token_count` thresholds to accommodate cloud server feature additions.

## [3.0.3] — 2026-04-12

### Dashboard Reliability + Automatic Background Indexing

#### Added

- **Background indexing orchestrator** — automatically builds and refreshes dependency graph, BM25 index, call graph, and route map in the background once a project root is known.
- **Dashboard status endpoint** — `GET /api/status` exposes per-index build states (`idle|building|ready|failed`) for progress display and troubleshooting.
- **Routes cache** — dashboard route map results are cached per project to avoid repeated scans.

#### Improved

- **Dashboard APIs are non-blocking** — graph/search/call-graph/routes endpoints return a `building` status instead of hanging while indexes are being built.
- **Dashboard UI** — views show “Indexing…” + auto-retry with backoff instead of confusing empty states or timeouts.
- **Auto-build on real usage** — MCP server triggers background builds when the project root is detected from `ctx_read` and also from `ctx_shell` (via effective working directory), without requiring manual reindex commands.

#### CI

- **AUR release hardening** — AUR job runs only when `AUR_SSH_KEY` is present, verifies SSH access up front, and fails loudly on auth issues.
- **Homebrew verification** — formula update step asserts the expected version + SHA are written before pushing.

#### Kiro IDE Support

- **Kiro steering file** — `lean-ctx init --agent kiro` and `lean-ctx setup` now create `.kiro/steering/lean-ctx.md` alongside the MCP config, ensuring Kiro uses lean-ctx tools instead of native equivalents.
- **Project-level detection** — `install_project_rules()` automatically creates the steering file when a `.kiro/` directory exists.

#### Fixed

- **`lean-ctx doctor` showed 9/10 instead of 10/10** — session state check was displayed but never counted towards the pass total.
- **Dashboard browser error on Linux** — suppressed Chromium stderr noise (`sharing_service.cc`) when opening dashboard via `xdg-open`.

## [3.0.2] — 2026-04-12

### Symbol Intelligence + Hybrid Semantic Search

#### Added — New MCP Tools

- **Symbol & outline navigation**
  - `ctx_symbol` — read a specific symbol by name (code span only)
  - `ctx_outline` — compact file outline (symbols + signatures)
- **Call graph navigation**
  - `ctx_callers` — find callers of a symbol
  - `ctx_callees` — list callees of a symbol
- **API surface extraction**
  - `ctx_routes` — extract HTTP routes/endpoints across common frameworks
- **Visualization**
  - `ctx_graph_diagram` — Mermaid diagram for dependency graph / call graph
- **Memory hygiene**
  - `ctx_compress_memory` — compress large memory/config markdown while preserving code fences/URLs

#### Improved — `ctx_semantic_search`

- **Search modes**: `bm25`, `dense`, `hybrid` (default)
- **Filters**: `languages` + `path_glob` to scope results
- **Automation**: auto-refreshes stale BM25 indexes; incremental embedding index updates
- **Performance**: process-level embedding engine cache (no repeated model load)

#### Fixed

- **Route extraction**: Spring-style Java methods with generic return types are now detected correctly.
- **Graph diagrams**: `depth` is now respected when filtering edges for `ctx_graph_diagram`.

## [3.0.1] — 2026-04-10

### LeanCTX Observatory — Real-Time Data Visualization Dashboard

#### Added — Observatory Dashboard (`lean-ctx dashboard`)

- **Event Bus** — New `EventKind`-based event system with ring buffer (1000 events) and JSONL persistence (`~/.lean-ctx/events.jsonl`) with auto-rotation at 10,000 lines. Captures `ToolCall`, `CacheHit`, `Compression`, `AgentAction`, `KnowledgeUpdate`, and `ThresholdShift` events in real time.
- **Live Observatory** — Real-time event feed showing all tool calls, cache hits, compression operations, agent actions, and knowledge updates with token savings, mode tags, and file paths. Filter by category (Reads, Shell, Search, Cache).
- **Knowledge Graph** — Interactive D3 force-directed graph visualizing project knowledge facts. Nodes sized by confidence, colored by category (Architecture, Testing, Debugging, etc.). Click nodes for detail panel showing temporal validity, confirmation count, and source session.
- **Dependency Map** — Force-directed visualization of file dependencies extracted via tree-sitter. Nodes sized by token count, colored by language, with edges representing import relationships. Smart edge resolution for module-style imports (`api::Server` → file path).
- **Compression Lab** — Side-by-side comparison of all compression modes (`map`, `signatures`, `aggressive`, `entropy`) for any file. Shows original content, compressed output, token savings percentage, and line reduction.
- **Agent World** — Multi-agent monitoring panel showing active agents, pending messages, shared contexts, agent types, roles, and last active times.
- **Bug Memory (Gotcha Tracker)** — Visual dashboard for auto-detected error patterns with severity, category, trigger/resolution, confidence scores, occurrence counts, and prevention statistics.
- **Search Explorer** — BM25 search index visualization with language distribution chart, top chunks by token count, and symbol-level detail.
- **Learning Curves** — Adaptive compression threshold visualization showing per-language entropy/Jaccard thresholds and compression outcome scatter plots (compression ratio vs. task success).

#### Added — Terminal TUI (`lean-ctx watch`)

- **`ratatui`-based Terminal UI** — Live event feed, file heatmap, token savings, and session stats in the terminal. Reads from `events.jsonl` with tail-based polling.

#### Added — Event Instrumentation

- `ctx_read`, `ctx_shell`, `ctx_search`, `ctx_tree` and all tools now emit `ToolCall` events with token counts, mode, duration, and path.
- Cache hits emit `CacheHit` events with saved token counts.
- `entropy_compress_adaptive()` emits `Compression` events with before/after line counts and strategy.
- `AgentRegistry.register()` emits `AgentAction` events.
- `ProjectKnowledge.remember()` emits `KnowledgeUpdate` events.
- `FeedbackStore` emits `ThresholdShift` events when learned thresholds change significantly.

#### Added — New Dashboard APIs

- `GET /api/events` — Latest 200 events from JSONL file (cross-process visibility).
- `GET /api/graph` — Full project dependency index.
- `GET /api/feedback` — Compression feedback outcomes and learned thresholds.
- `GET /api/session` — Current session state.
- `GET /api/search-index` — BM25 index summary with language distribution and top chunks.
- `GET /api/compression-demo?path=<file>` — On-demand compression of any file through all modes with original content preview.

#### Fixed

- **Live Observatory** showed "unknown" for all events due to flat vs. nested `kind` object mismatch — implemented `flattenEvent()` parser supporting all 6 event types.
- **Agent World** status comparison was case-sensitive (`Active` vs `active`) — now case-insensitive.
- **Learning Curves** scatter plot showed 0 for x-axis — now computes compression ratio from `tokens_saved / tokens_original` when `compression_ratio` field is absent.
- **Compression Lab** failed to load files — added `rust/` prefix fallback for path resolution and `original` content field in API response.
- **Dependency Map** edges not connecting — added module-to-file path resolution for `api::Server`-style import targets.

---

## [3.0.0] — 2026-04-10

### Major Release: Waves 1-5 — Intelligence Engine, Knowledge Graph, A2A Protocol, Adaptive Compression

This is a **major release** bringing lean-ctx from 28 to **34 MCP tools**, adding 8 read modes (new: `task`), persistent knowledge with temporal facts, multi-agent orchestration (A2A protocol), adaptive compression with Thompson Sampling bandits, and a complete fix for the context dropout bug (#73).

---

#### Wave 1 — Neural Token Optimization & Graph-Aware Filtering

- **Neural token optimizer** — Attention-weighted compression that preserves high-information-density lines using Shannon entropy scoring with configurable thresholds.
- **Graph-aware Information Bottleneck filter** — Integrates the project knowledge graph into `task` mode filtering, preserving lines that reference known entities (functions, types, modules) from the dependency graph.
- **Task relevance scoring** — Renamed `information_bottleneck_filter` → `graph_aware_ib_filter` with KG-powered entity recognition for smarter context selection.

#### Wave 2 — Context Reordering & Entropy Engine

- **LITM-aware context reordering** — Reorders compressed output using a U-curve attention model (Lost-in-the-Middle), placing high-importance content at the start and end of context windows where LLM attention is strongest.
- **Adaptive entropy thresholds** — Per-language BPE entropy thresholds with Kolmogorov complexity adjustment that auto-tune based on file characteristics.
- **`task` read mode** — New compression mode that filters content through the Information Bottleneck principle, preserving only task-relevant lines. Achieves 65-85% savings while maintaining semantic completeness.

#### Wave 3 — Persistent Knowledge & Episodic Memory

- **`ctx_knowledge` tool** — Persistent project knowledge store with temporal validity, confidence decay, and contradiction detection. Actions: `remember`, `recall`, `timeline`, `rooms`, `search`, `wakeup`.
- **Episodic memory** — Facts have temporal validity (`valid_from`/`valid_until`) and confidence scores that decay over time for unused knowledge.
- **Procedural memory** — Cross-session knowledge that automatically surfaces relevant facts based on the current task context.
- **Contradiction detection** — When storing a new fact that contradicts an existing one in the same category, the old fact is automatically superseded.

#### Wave 4 — A2A Protocol & Multi-Agent Orchestration

- **`ctx_task` tool** — Google A2A (Agent-to-Agent) protocol implementation with full task lifecycle: `create`, `assign`, `update`, `complete`, `cancel`, `list`, `get`.
- **`ctx_cost` tool** — Cost attribution per agent with token tracking. Actions: `record`, `summary`, `by_agent`, `reset`.
- **`ctx_heatmap` tool** — File access heatmap tracking read counts, compression ratios, and access patterns. Actions: `show`, `hot`, `cold`, `reset`.
- **`ctx_impact` tool** — Measures the impact of code changes by analyzing dependency chains in the knowledge graph.
- **`ctx_architecture` tool** — Generates architectural overviews from the project's dependency graph and module structure.
- **Agent Card** — `.well-known/agent.json` endpoint for A2A agent discovery with capabilities, supported modes, and rate limits.
- **Rate limiter** — Per-agent sliding window rate limiting (configurable, default 100 req/min).

#### Wave 5 — Adaptive Compression (ACON + Bandits)

- **ACON feedback loop** — Adaptive Compression via Outcome Normalization. Tracks compression outcomes (quality signals from LLM responses) and adjusts thresholds automatically.
- **Thompson Sampling bandits** — Multi-armed bandit approach for selecting optimal compression parameters per file type and language. Uses Beta distributions with configurable priors.
- **Quality signal detection** — Automatically detects quality signals in LLM responses (re-reads, error patterns, follow-up questions) to feed the ACON loop.
- **`ctx_shell` cwd tracking** — Shell working directory is now tracked across calls. `cd` commands are parsed and persisted in the session. New `cwd` parameter for explicit directory control.

#### Fix: Context Dropout Bug (#73)

All five root causes of the "lean-ctx loses context after initial read phase" bug have been fixed:

- **Monorepo-aware `project_root`** — `detect_project_root()` now finds the outermost ancestor with a project marker (`.git`, `Cargo.toml`, `package.json`, `go.work`, `pnpm-workspace.yaml`, `nx.json`, `turbo.json`, etc.), not the nearest `.git`.
- **`ctx_shell` cwd persistence** — New `shell_cwd` field in session state. `cd` commands are parsed and the working directory persists across `ctx_shell` calls. Priority: explicit `cwd` arg → session `shell_cwd` → `project_root` → process cwd.
- **`ctx_overview`/`ctx_preload` root fallback** — Both tools now fall back to `session.project_root` when no `path` parameter is given (previously defaulted to server process cwd).
- **Relative path resolution** — All 15+ path-based tools now use `resolve_path()` which tries: original path → `project_root` + relative → `shell_cwd` + relative → fallback.
- **Windows shell chaining** — `;` in commands is automatically converted to `&&` when running under `cmd.exe`.

#### Improved — Diagnostics

- **`lean-ctx doctor`** — New session state check showing `project_root`, `shell_cwd`, and session version.

#### Stats

- **34 MCP tools** (was 28)
- **8 read modes** (was 7, new: `task`)
- **656+ unit tests** passing
- **14 integration tests** passing
- **24 supported editors/AI tools**

## [2.21.11] — 2026-04-09

### Fix: Dashboard, Doctor, and MCP Reliability (#72)

#### Fixed — Doctor gave false positives for broken MCP configs
- **MCP JSON validation** — `doctor` now validates the actual JSON structure of each MCP config file instead of just checking for the string "lean-ctx". Checks for `mcpServers` → `lean-ctx` → `command` fields, verifies the binary path exists, and reports **per-IDE** status (valid vs. broken configs).
- **Honest stats check** — A missing `stats.json` is now reported as a warning ("MCP server has not been used yet") instead of counting as a passed check.

#### Fixed — Dashboard showed empty state without guidance
- The empty state now includes an actionable **troubleshooting checklist** with IDE-specific steps (Cursor reload, Claude Code init, config validation).

#### Fixed — No session created until first tool call batch
- A session is now created immediately on MCP `initialize`, so `doctor --report` always shows session info even before any tools are used.

#### Fixed — Tool calls only logged when >100ms
- All tool calls are now logged regardless of duration. Previously, fast calls were silently dropped, making the tool call log appear empty.

#### Fixed — macOS binary hangs at `_dyld_start` after install
- On macOS, copying the binary (via `cp`, `install`, or download) could strip the ad-hoc code signature, causing the dynamic linker to hang indefinitely on startup. Both `install.sh` and the self-updater now run `xattr -cr` + `codesign --force --sign -` after placing the binary.

## [2.21.10] — 2026-04-09

### Fix: Auth/Device Code Flow Output Preserved

#### Fixed — OAuth device code output no longer compressed (#71)
- **Auth flow detection** — New `contains_auth_flow()` function detects OAuth device code flow output using a two-tier approach:
  - **Strong signals** (match alone): `devicelogin`, `deviceauth`, `device_code`, `device code`, `device-code`, `verification_uri`, `user_code`, `one-time code`
  - **Weak signals** (require URL in same output): `enter the code`, `use a web browser to open`, `verification code`, `waiting for authentication`, `authorize this device`, and 10 more patterns
- **Shell hook passthrough** — 21 auth commands added to `BUILTIN_PASSTHROUGH`: `az login`, `gh auth`, `gcloud auth`, `aws sso`, `firebase login`, `vercel login`, `heroku login`, `flyctl auth`, `vault login`, `kubelogin`, `--use-device-code`, and more. These bypass compression entirely.
- **MCP tool passthrough** — `ctx_shell::handle()` now checks output for auth flows before compression. If detected, full output is preserved with a `[lean-ctx: auth/device-code flow detected]` note.
- **Shell hook buffered path** — `compress_if_beneficial()` also checks for auth flows before any compression, covering the `exec_buffered` path when stdout is not a TTY.

#### Impact
Previously, when Codex or Claude Code ran an auth command (e.g. `az login --use-device-code`), the device code was hidden from the user because lean-ctx compressed the output. Now the full output including auth codes is preserved.

**Workaround for older versions:** Add `excluded_commands = ["az login"]` to `~/.lean-ctx/config.toml`, or prefix commands with `LEAN_CTX_DISABLED=1`.

## [2.21.9] — 2026-04-09

### First-Class MCP Support for Pi Coding Agent

#### Added — pi-lean-ctx v2.0.0 with Embedded MCP Bridge
- **Embedded MCP client** — pi-lean-ctx now spawns the lean-ctx binary as an MCP server (JSON-RPC over stdio) and registers all 20+ advanced tools (ctx_session, ctx_knowledge, ctx_semantic_search, ctx_overview, ctx_compress, ctx_metrics, ctx_agent, ctx_graph, ctx_discover, ctx_context, ctx_preload, ctx_delta, ctx_edit, ctx_dedup, ctx_fill, ctx_intent, ctx_response, ctx_wrapped, ctx_benchmark, ctx_analyze, ctx_cache, ctx_execute) as native Pi tools.
- **Automatic pi-mcp-adapter compatibility** — If lean-ctx is already configured in `~/.pi/agent/mcp.json` (via pi-mcp-adapter), the embedded bridge is skipped to avoid duplicate tool registration.
- **Dynamic tool discovery** — Tool schemas come directly from the MCP server at runtime, not hardcoded. The `disabled_tools` config is respected.
- **Auto-reconnect** — If the MCP server process crashes, the bridge reconnects automatically (3 attempts with exponential backoff). CLI-based tools (bash, read, grep, find, ls) continue working regardless.
- **`/lean-ctx` command enhanced** — Now shows binary path, MCP bridge status (embedded vs. adapter), and list of registered MCP tools.

#### Added — Pi auto-detection in `lean-ctx setup`
- **Pi Coding Agent** is now auto-detected alongside Cursor, Claude Code, VS Code, Zed, and all other supported editors. Running `lean-ctx setup` writes `~/.pi/agent/mcp.json` automatically.
- **`lean-ctx init --agent pi`** now also writes the MCP server config to `~/.pi/agent/mcp.json` with `lifecycle: lazy` and `directTools: true`.

#### Improved — Pi diagnostics
- **`lean-ctx doctor`** now shows three Pi states: "pi-lean-ctx + MCP configured", "pi-lean-ctx installed (embedded bridge active)", or "not installed".

#### Documentation
- **README** for pi-lean-ctx completely rewritten with MCP tools table, pi-mcp-adapter compatibility guide, and `disabled_tools` configuration.
- **PI_AGENTS.md** template updated with MCP tools section.

## [2.21.8] — 2026-04-09

### Self-Updater Shell Alias Refresh + Thinking Budget Tuning

#### Fixed — `lean-ctx update` now refreshes shell aliases automatically
- **Shell alias auto-refresh** — `post_update_refresh()` now detects all shell configs (`~/.zshrc`, `~/.bashrc`, `config.fish`, PowerShell profile) with lean-ctx hooks and rewrites them with the latest `_lc()` function. Previously, `lean-ctx update` only refreshed AI tool hooks (Claude, Cursor, Gemini, Codex) but left shell aliases untouched, meaning users had to manually run `lean-ctx setup` to get new hook logic like the pipe guard.
- **Multi-shell support** — If a user has hooks in both `.zshrc` and `.bashrc`, both are now updated (previously only the first match was handled).
- **Post-update message** — Now explicitly tells users to `source ~/.zshrc` or restart their terminal.

#### Changed — Thinking Budget Tuning
- `FixBug` intent: Minimal → **Medium** (bug fixes benefit from deeper reasoning)
- `Explore` intent: Medium → **Minimal** (exploration is lightweight)
- `Debug` intent: Medium → **Trace** (debugging needs full chain-of-thought)
- `Review` intent: Medium → **Trace** (code review needs thorough analysis)

#### Improved — README & Deploy Checklist
- **README** — Added "Updating lean-ctx" section with all update methods, added pipe guard troubleshooting entry.
- **Deploy checklist** — Added "Shell Hook Refresh", "README / GitHub Updates" sections, and two new common pitfalls.

## [2.21.7] — 2026-04-09

### Cleanup + Website Redesign

#### Changed — Remove Hook E2E Test Suite
- **Removed `hook_e2e_tests.rs`** — The hook E2E test file and its corresponding CI workflow (`hook-integration`) have been removed. The pipe guard behavior is already covered by the integration tests in `integration_tests.rs` and the unit tests in `cli.rs`. This eliminates a redundant CI job that depended on `generate_rewrite_script`, simplifying the test matrix.

#### Changed — Website: LeanCTL Section Redesigned
- **Consistent page design** — The LeanCTL ecosystem section on the homepage now uses the same visual patterns (compare-cards, layer-cards, stats-grid) as the rest of the page, replacing the custom TUI terminal mockup with ~150 lines of dedicated CSS.
- **Real product facts** — Compare cards show concrete token savings from leanctl.com (4,200 → 48 tokens for file reads, 847 → 42 for test output, 4,200 → ~13 for re-reads).
- **Three feature cards** — "23 Built-in Tools", "Thinking Steering", "Bring Your Own Key" in the standard layer-card layout.
- **Stats grid** — "up to 90% savings", "23 tools", "8 compression modes", "0 data sent to us".

#### Changed — Navigation: Dedicated Ecosystem Dropdown
- **New top-level nav item** — "Ecosystem" mega dropdown with two columns: "AI Agents" (LeanCTL) and "Community" (GitHub, Discord, Blog).
- **Product dropdown cleaned** — Removed the ecosystem column from the Product mega dropdown (now 3 columns instead of 4).
- **Mobile menu updated** — Ecosystem section with LeanCTL, GitHub, Discord links.

#### i18n
- All 11 locale files updated with new ecosystem keys (en/de with translations, others with English fallbacks).

## [2.21.6] — 2026-04-08

### Shell Hook Pipe Guard — Fix `curl | sh` Broken by lean-ctx

#### Fixed — Piped commands corrupted by lean-ctx compression
- **Pipe guard for Bash/Zsh** — `_lc()` now checks `[ ! -t 1 ]` (stdout is not a terminal) before routing through lean-ctx. When piped (e.g. `curl -fsSL https://example.com/install.sh | sh`), commands run directly without compression. Previously, lean-ctx would buffer and compress the output, corrupting install scripts and other piped data.
- **Pipe guard for Fish** — `_lc` now checks `not isatty stdout` before routing through lean-ctx.
- **Pipe guard for PowerShell** — `_lc` now checks `[Console]::IsOutputRedirected` before routing through lean-ctx.

#### Important
After updating, run `lean-ctx init` to regenerate the shell hooks with the pipe guard. Or open a new terminal tab.

#### Testing
- 5 new E2E tests for pipe-guard behavior and piped output preservation.
- 3 new unit tests verifying pipe-guard presence in all shell hook variants (Bash, Fish, PowerShell).
- All 677 tests passing, zero clippy warnings.

## [2.21.5] — 2026-04-08

### Windows Updater Infinite Loop Fix (#69)

#### Fixed — Updater enters infinite loop with 100% CPU on Windows
- **Replaced `timeout /t` with `ping` delay** — The deferred update `.bat` script used `timeout /t 1 /nobreak` for delays. On Windows systems with GNU coreutils in PATH (Git Bash, Cygwin, MSYS2), the GNU `timeout` binary takes precedence over the Windows built-in, fails instantly with "invalid time interval '/t'", and causes a tight retry loop at 100% CPU. Now uses `ping 127.0.0.1 -n 2 >nul` which works on every Windows system regardless of PATH.
- **Added retry limit (60 attempts)** — The script now exits with an error message after 60 failed attempts (~60 seconds) instead of looping indefinitely. Cleans up the pending binary on timeout.
- **Extracted `generate_update_script()` as public function** for testability.

#### Testing
- 10 new unit tests covering: no `timeout` command usage, `ping` delay, retry limit, counter increment, timeout exit, pending file cleanup, path substitution (incl. spaces), batch syntax validity, rollback on failure.
- All 669 tests passing, zero clippy warnings.

## [2.21.4] — 2026-04-08

### Windows Shell Fix + Antigravity Support

#### Fixed — Windows: `ctx_shell` fails with "& was unexpected at this time"
- **PowerShell always preferred** — On Windows, `find_real_shell()` now always attempts to locate PowerShell (`pwsh.exe` or `powershell.exe`) before falling back to `cmd.exe`. Previously, PowerShell was only used if `PSModulePath` was set — but when IDEs (VS Code, Codex, Antigravity) spawn the MCP server, this env var is often absent. Since AI agents send bash-like syntax (`&&`, pipes, subshells), `cmd.exe` cannot parse these commands. This was the root cause of "& was unexpected at this time" errors reported by Windows users.
- **`LEAN_CTX_SHELL` override** — Users can set `LEAN_CTX_SHELL=powershell.exe` (or any shell path) to force a specific shell, bypassing all detection logic.

#### Added — `antigravity` agent support
- **`lean-ctx init --agent antigravity`** — Now recognized as alias for `gemini`, creating the same hook scripts and settings under `~/.gemini/`. Previously, Antigravity users had to know to use `--agent gemini` or run `lean-ctx setup`.

#### Testing
- 19 new E2E tests covering shell detection, `LEAN_CTX_SHELL` override, shell command execution (pipes, `&&`, subshells, env vars), agent init (antigravity alias, unknown agent handling), Windows path handling in generated scripts, and bash script execution with Windows binary paths.
- 10 new unit tests for Windows shell flag detection and shell detection logic.
- All 659 tests passing, zero clippy warnings.

## [2.21.3] — 2026-04-08

### Robust Hook Escaping + Auto-Context Fix

#### Fixed — Commands with Embedded Quotes Truncated
- **JSON parser rewrite** — Hook scripts and Rust handler now correctly parse JSON values containing escaped quotes (e.g. `curl -H "Authorization: Bearer token"`). Previously, the naive `[^"]*` regex stopped at the first `\"` inside the value, truncating the command. Now uses `([^"\\]|\\.)*` pattern with proper unescape pass. Affects both bash scripts and Rust `extract_json_field`.
- **Double-escaping for rewrites** — Rewrite output now applies two escaping passes: shell-escape (for the `-c "..."` wrapper) then JSON-escape (for the hook protocol). Previously, only one pass was applied, causing inner quotes to break both shell and JSON parsing.

#### Fixed — Auto-Context Noise from Wrong Project (#62 Issue 4)
- **Project root guard** — `session_lifecycle_pre_hook` and `enrich_after_read` now require a known, non-trivial `project_root` before triggering auto-context. Previously, when `project_root` was `None` or `"."`, the autonomy system would run `ctx_overview` on the MCP server's working directory (often a completely different project), injecting irrelevant "AUTO CONTEXT" blocks into responses.

#### Improved — Cache Hit Message Clarity (#62 Issue 3)
- **Actionable stub** — Cache hit responses now include guidance: `"File already in context from previous read. Use fresh=true to re-read if content needed again."` Previously, the terse `F1=main.rs cached 2t 4L` stub left AI agents confused about what to do next.

#### Housekeeping
- Redirect scripts reduced to minimal `exit 0` (removed ~30 lines of dead `is_binary`/`FILE_PATH` parsing code that was never reached).
- 4 new unit tests for escaped-quote JSON parsing and double-escaping.
- 1 new integration test for auto-context project_root guard.
- All 611 tests passing, zero clippy warnings.

## [2.21.2] — 2026-04-08

### Critical Hook Fixes — Production Quality (Discussion #62)

#### Fixed — Pipe Commands Broken in Shell Hook
- **Pipe quoting fix** — Hook rewrite now properly quotes commands containing pipes. Previously `curl ... | python3 -m json.tool` was rewritten as `lean-ctx -c curl ... | python3 ...` (pipe interpreted by shell). Now correctly produces `lean-ctx -c "curl ... | python3 ..."`. This also fixes the `command not found: _lc` errors reported by users.

#### Fixed — Read/Grep/ListFiles Blocked by Hook (#62)
- **Removed tool blocking** — The redirect hook no longer denies native Read, Grep, or ListFiles tools. This was causing Claude Code's Edit tool to fail ("File has not been read yet") because Edit requires a prior native Read. Native tools now pass through freely. The MCP system instructions still guide the AI to prefer `ctx_read`/`ctx_search`/`ctx_tree`, but blocking is removed.

#### Fixed — `find` Command Glob Pattern Support
- **Glob patterns** — `lean-ctx find "*.toml"` now correctly uses glob matching instead of literal substring search. Added `glob` crate dependency.

#### Changed — README
- **RTK** — Corrected "RTK" references to full name "Rust Token Killer" throughout README and FAQ section.

#### Housekeeping
- Removed ~180 lines of dead code from `hook_handlers.rs` (unused glob matching, binary detection, path exclusion functions that were orphaned by the redirect removal).
- Added 3 new unit tests for hook rewrite quoting behavior.
- All 504 tests passing, zero clippy warnings.

## [2.21.1] — 2026-04-08

### CLI File Caching

#### Added — Persistent CLI Read Cache (#65)
- **File-based CLI caching** — `lean-ctx read <file>` now caches file content to `~/.lean-ctx/cli-cache/cache.json`. Second and subsequent reads of unchanged files return a compact ~13-token cache-hit response instead of the full file content. This directly addresses Issue #65 (pi-lean-ctx zero cache hits) by enabling caching for CLI-mode integrations that don't use the MCP server.
- **Cache management** — New `lean-ctx cache` subcommand with `stats`, `clear`, and `invalidate <path>` actions.
- **`--fresh` / `--no-cache` flag** — Bypass the CLI cache for a single read when needed.
- **5-minute TTL** — Cache entries expire after 300 seconds, matching the MCP server cache behavior.
- **MD5 change detection** — Files are re-read when their content changes, even within the TTL window.
- **Max 200 entries** — Oldest entries are evicted when the cache exceeds capacity.
- 6 new unit tests including integration test for full cache lifecycle.

#### Fixed — Missing Module Registrations
- Registered `sandbox` and `loop_detection` modules that were present on disk but missing from `core/mod.rs`.

## [2.21.0] — 2026-04-08

### Binary File Passthrough, Disabled Tools, Community Contributions

#### Fixed — Hook Blocks Image Viewing (#67)
- **Binary file passthrough** — Hook redirect now detects binary files (images, PDFs, archives, fonts, videos, compiled files) by extension and passes them through to the native Read tool. Previously, the hook would deny all `read_file` calls when lean-ctx was running, which blocked AI agents from viewing screenshots and images.
- Updated both Rust `handle_redirect()` and all bash hook scripts (Claude, Cursor, Gemini CLI) with the same binary extension check.

#### Added — Disabled Tools Config (#66, @DustinReynoldsPE)
- **`disabled_tools`** config field — Exclude unused tools from the MCP tool list to reduce token overhead from tool definitions. Configure via `~/.lean-ctx/config.toml` or `LEAN_CTX_DISABLED_TOOLS` env var (comma-separated).
- Example: `disabled_tools = ["ctx_benchmark", "ctx_metrics", "ctx_analyze", "ctx_wrapped"]`
- 10 new tests covering parsing, TOML deserialization, and filtering logic.

#### Closed — Cache Hits Documentation (#65)
- Clarified that file caching requires MCP server mode (`ctx_read`), not shell hook mode (`lean-ctx -c`). Shell hooks compress command output only; the MCP server provides file caching with ~13 token re-reads.

## [2.20.0] — 2026-04-07

### Sandbox Execution, Progressive Throttling, Compaction Recovery

#### Added — Sandbox Code Execution
- **`ctx_execute`** — New MCP tool that runs code in 11 languages (JavaScript, TypeScript, Python, Shell, Ruby, Go, Rust, PHP, Perl, R, Elixir) in an isolated subprocess. Only stdout enters the context window — raw data never leaves the sandbox. Supports `action=batch` for multiple scripts in one call, and `action=file` to process files in sandbox with auto-detected language.
- **Smart truncation** — Large outputs (>32 KB) are truncated with head (60%) + tail (40%) preservation, keeping both setup context and error messages visible.
- **`LEAN_CTX_SANDBOX=1` env** — Set in all sandbox processes for detection by user code.
- **Timeout support** — Default 30s, configurable per-call.

#### Added — Progressive Throttling (Loop Detection)
- **Automatic agent loop detection** — Tracks tool call fingerprints within a 5-minute sliding window. Calls 1-3: normal. Calls 4-8: reduced results + warning. Calls 9-12: stronger warning. Calls 13+: blocked with suggestion to use `ctx_batch_execute` or vary approach.
- **Deterministic fingerprinting** — JSON args are canonicalized (key-sorted) before hashing, so `{path: "a", mode: "b"}` and `{mode: "b", path: "a"}` are treated as the same call.
- **Per-tool tracking** — Different tools with different args are tracked independently.

#### Added — Compaction Recovery
- **`ctx_session(action=snapshot)`** — Builds a priority-tiered XML snapshot (~2 KB max) of the current session state including task, modified files, decisions, findings, progress, test results, and stats. Saved to `~/.lean-ctx/sessions/{id}_snapshot.txt`.
- **`ctx_session(action=restore)`** — Rebuilds session state from the most recent compaction snapshot. When the context window fills up and the agent compacts, the snapshot allows seamless continuation.
- **Priority tiers** — Task and files (P1) are always included. Decisions and findings (P2) next. Tests, next steps, and stats (P3/P4) are dropped first if the 2 KB budget is tight.

## [2.19.2] — 2026-04-07

### Fixed
- **Gemini CLI hook schema** — Fixed "Discarding invalid hook definition for BeforeTool" error. Hook definitions now include the required `"type": "command"` field and nested `"hooks"` array structure expected by the Gemini CLI validator. Existing configs without `"type"` are automatically migrated. (#63)
- **Remote dashboard auth** — Fixed dashboard returning `{"error":"unauthorized"}` when accessed remotely via browser. Auth is now only enforced on `/api/*` endpoints. HTML pages load freely, with the bearer token automatically injected into API calls. Browser URL with `?token=` query parameter is printed on startup for easy remote access. (#64)

## [2.19.1] — 2026-04-07

### Fixed
- **Cursor hooks.json format** — Fixed invalid hooks.json that caused "Config version must be a number; Config hooks must be an object" error in Cursor. Now generates correct format with `"version": 1` and hooks as an object with `preToolUse` key instead of array. Existing broken configs are automatically migrated on next `lean-ctx install cursor` or MCP server start.
- **cargo publish workflow** — Added `--allow-dirty` to release pipeline to prevent publish failures from checkout artifacts

## [2.19.0] — 2026-04-07

### Temporal Knowledge, Contradiction Detection, Agent Diaries & Cross-Session Search

#### Added — Knowledge Intelligence
- **Temporal facts** — All facts now track `valid_from`/`valid_until` timestamps. When a high-confidence fact changes, the old value is archived (not deleted) with full history
- **Contradiction detection** — `ctx_knowledge(action=remember)` automatically detects when a new fact conflicts with an existing high-confidence fact, reporting severity (low/medium/high) and resolution
- **Confirmation tracking** — Facts that are re-asserted gain increasing `confirmation_count`, boosting their reliability score
- **Knowledge rooms** — `ctx_knowledge(action=rooms)` lists all knowledge categories (rooms) with fact counts, providing a MemPalace-like structured overview
- **Timeline view** — `ctx_knowledge(action=timeline, category="...")` shows the full version history of facts in a category, including archived values with validity ranges
- **Cross-session search** — `ctx_knowledge(action=search, query="...")` searches across ALL projects and ALL past sessions for matching facts, findings, and decisions
- **Wake-up briefing** — `ctx_knowledge(action=wakeup)` returns a compact AAAK-formatted briefing of the most important project facts
- **AAAK format** — Compact knowledge representation (`CATEGORY:key=value★★★|key2=value2★★`) used in LLM instructions instead of verbose prose, saving ~60% tokens

#### Added — Agent Diaries
- **Persistent agent diaries** — `ctx_agent(action=diary, category=discovery|decision|blocker|progress|insight)` logs structured entries that persist across sessions at `~/.lean-ctx/agents/diaries/`
- **Diary recall** — `ctx_agent(action=recall_diary)` shows the 10 most recent diary entries for an agent with timestamps and context
- **Diary listing** — `ctx_agent(action=diaries)` lists all agent diaries across the system with entry counts and last-updated times

#### Added — Wake-Up Context
- **ctx_overview wake-up briefing** — `ctx_overview` now automatically includes a compact briefing at session start: top project facts (AAAK), last task, recent decisions, and active agents — zero configuration needed

#### Changed
- **Knowledge block in LLM instructions** now uses AAAK compact format instead of verbose prose, reducing knowledge injection tokens by ~60%
- **MCP tool descriptions** updated for `ctx_knowledge` (12 actions) and `ctx_agent` (11 actions) to document all new capabilities

## [2.18.1] — 2026-04-07

### Code Quality & Security Hardening

#### Fixed
- **Shell injection in CLI** — `lean-ctx grep` and `lean-ctx find` no longer shell-interpolate user input; replaced with pure Rust implementation using `ignore::WalkBuilder` + `regex`
- **Panic in `report_gotcha`** — `unwrap()` after `add_or_merge` could panic when gotcha store exceeds capacity (100 entries) and the new entry gets evicted; now returns `Option<&Gotcha>` safely
- **Broken `FilterEngine` cache** — Removed dead `get_or_load()` method that stored empty rules in a `Mutex` and was never called; `CACHED_ENGINE` static removed
- **`unwrap()` after `is_some()` pattern** — Replaced fragile double-lookup + `unwrap()` with idiomatic `if let Some()` / `match` in `ctx_read`, `ctx_smart_read`, and `ctx_delta`
- **`graph` CLI argument parsing** — `lean-ctx graph build /path` now correctly separates action from path argument

#### Added
- **`lean-ctx graph` CLI command** — Build the project dependency graph from the command line (`lean-ctx graph [build] [path]`); previously only available via MCP `ctx_graph` tool
- **Consolidated `detect_project_root`** — Single implementation in `core::protocol` replacing 3 duplicate copies across `server.rs`, `ctx_read.rs`, and `dashboard/mod.rs`

#### Changed
- **Tokio features trimmed** — `features = ["full"]` replaced with 8 specific features (`rt`, `rt-multi-thread`, `macros`, `io-std`, `io-util`, `net`, `sync`, `time`), reducing compile time and binary size
- **Security workflow updated** — `security-check.yml` now correctly documents `ureq` as the allowed HTTP client (for opt-in cloud sync, updates, error reports) instead of claiming "no network"

## [2.18.0] — 2026-04-07

### Multi-Agent Context Sharing, Semantic Caching, Dashboard & Editor Integrations

#### Added — Multi-Agent
- **`ctx_share` tool** (28th MCP tool) — Share cached file contexts between agents. Actions: `push`, `pull`, `list`, `clear`
- **`ctx_agent` handoff action** — Transfer a task to another agent with a summary message, automatically marks the handing-off agent as finished
- **`ctx_agent` sync action** — Combined overview of active agents, pending messages, and shared contexts
- **`lctx --agents` flag** — Launch multiple agents in parallel: `lctx --agents claude,gemini` starts both in the background with shared context
- **Dashboard `/api/agents` enhancement** — Returns structured JSON with active agents, pending messages, and shared context count

#### Added — Intent & Semantic Intelligence
- **Multi-intent detection** — `ctx_intent` now detects compound queries ("fix X and then test Y") and splits them into sub-intents with individual classifications
- **Complexity classification** — `ctx_intent` returns task complexity (mechanical/standard/architectural) based on query analysis, target count, and cross-cutting keywords
- **Heat-ranked file strategy** — `ctx_intent` file discovery ranks results by heat score (token density + graph connectivity)
- **Semantic cache** — TF-IDF + cosine similarity index for finding semantically similar files across reads. Persistent at `~/.lean-ctx/semantic_cache/`. Cache warming suggestions based on access patterns. Hints shown on `ctx_read` cache misses

#### Added — Dashboard & CLI
- **`lean-ctx heatmap`** — New CLI command for context heat map visualization with color-coded token counts and graph connections
- **Dashboard authentication** — Bearer token auth for `/api/*` endpoints, token generated on first launch at `~/.lean-ctx/dashboard_token`
- **Heatmap API** — `GET /api/heatmap` returns project-wide file heat scores as JSON

#### Added — Editor Integrations
- **VS Code Extension** (`packages/vscode-lean-ctx`) — Status bar token savings, one-click setup, MCP auto-config for GitHub Copilot, command palette (setup, doctor, gain, dashboard, heatmap)
- **Chrome Extension** (`packages/chrome-lean-ctx`) — Manifest V3, auto-compress pastes in ChatGPT, Claude, Gemini. Native messaging bridge for full compression, fallback for comment/whitespace removal

#### Changed
- MCP tool count: 25 → 28 across all documentation, READMEs, SKILL.md, and 11 website locales


## [2.17.6] — 2026-04-07

### Feature: Crush Support (#61)

#### Added
- **Crush integration** — `lean-ctx init --agent crush` configures MCP in `~/.config/crush/crush.json` with the Crush-specific `"mcp"` key format (instead of `"mcpServers"`)
- **Auto-detection** — `lean-ctx setup` and `lean-ctx doctor` now detect Crush installations
- **Rules injection** — `lean-ctx rules` creates `~/.config/crush/rules/lean-ctx.md` when Crush is installed
- **Prompt generator** — Website getting-started page includes Crush with manual config instructions
- **Compatibility page** — Crush listed in all compatibility matrices across 11 languages

## [2.17.5] — 2026-04-06

### Fix: ctx_shell Input Validation (#50)

#### Added
- **File-write command blocking** — `ctx_shell` now detects and rejects shell redirects (`>`, `>>`), heredocs (`<< EOF`), and `tee` commands. Returns a clear error redirecting to the native Write tool
- **Command size limit** — Rejects commands over 8KB, preventing oversized heredocs from corrupting the MCP protocol stream
- **Quote-aware redirect parsing** — Redirect detection respects single/double quotes, ignores `2>` (stderr) and `> /dev/null`

This prevents the cascading failure reported in #50:
Oversized `ctx_shell` → API Error 400 → MCP stream corruption → "path is required" → MCP stops

## [2.17.4] — 2026-04-06

### Feature: Hook Redirect Path Exclusion + Automated Publishing

#### Added
- **Path exclusion for hook redirect** (#60) — Exclude specific paths from PreToolUse redirect hook. Paths matching patterns bypass the redirect and allow native Read/Grep/ListFiles to proceed
  - Config: `redirect_exclude = [".wolf/**", ".claude/**", "*.json"]` in `~/.lean-ctx/config.toml`
  - Env var: `LEAN_CTX_HOOK_EXCLUDE=".wolf/**,.claude/**"` (takes precedence)
  - Glob patterns support `*`, `?`, and `**` (recursive directory match)
- **Automated crates.io publishing** — `cargo publish` runs automatically after GitHub Release
- **Automated npm publishing** — `lean-ctx-bin` and `pi-lean-ctx` published automatically

## [2.17.3] — 2026-04-06

### Fix: MCP Stdout Pollution on Windows

#### Fixed
- **Windows MCP "not valid JSON" error** — `println!("Installed...")` messages in `install_claude/cursor/gemini_hook_config` polluted stdout during MCP server initialization, breaking JSON-RPC protocol. Now suppressed via `mcp_server_quiet_mode()` guard. (Fixes Lorenzo Rossi's report on Discord)

#### Changed
- **LanguageSwitcher position** — Moved to the right of the "Get Started" button in the header
- **Token Guardian Buddy** — Now shown inline in `lean-ctx gain` output when enabled
- **Bug Memory stats** — Active gotchas and prevention stats shown in `lean-ctx gain`
- **Helpful footer** — `lean-ctx gain` now shows links to `report-issue`, `contribute`, and `gotchas`

## [2.17.2] — 2026-04-06

### Fix: Cross-Platform Hook Handlers

#### Fixed
- **Windows: PreToolUse hook errors** — Agent hooks (Claude Code, Cursor, Gemini) no longer require Bash. Hook logic is now implemented natively in the lean-ctx binary via `lean-ctx hook rewrite` and `lean-ctx hook redirect` (#49)
- **"Stuck in file reading"** — Fixed hook redirect loop where denied Read/Grep tools caused repeated retries when the MCP server wasn't properly connected
- **Hook auto-migration** — Existing `.sh`-based hook configs are automatically upgraded to native binary commands on next MCP server start

#### Changed
- Hook configs now point to `lean-ctx hook rewrite` / `lean-ctx hook redirect` instead of `.sh` scripts
- `refresh_installed_hooks()` also updates hook configs (not just scripts) to ensure migration

## [2.17.1] — 2026-04-05

### Token Guardian Buddy — Data-Driven ASCII Companion

#### Added
- **Token Guardian Buddy** — Tamagotchi-style companion that evolves based on real usage metrics (tokens saved, commands, bugs prevented)
- **Procedural ASCII avatar generation** — Over 69 million unique creature combinations from 8 modular body parts (head, eyes, mouth, ears, body, legs, tail, markings)
- **Deterministic identity** — Each user gets a unique, persistent buddy based on their system seed
- **XP & leveling system** — XP calculated from tokens saved, commands issued, and bugs prevented; level derived via `sqrt(xp / 50)`
- **Rarity tiers** — Egg → Common → Uncommon → Rare → Epic → Legendary, based on lifetime tokens saved
- **Mood system** — Dynamic mood (Happy, Focused, Tired, Excited, Zen) derived from compression rate, errors, bugs prevented, and streak
- **RPG stats** — Compression, Vigilance, Endurance, Wisdom, Experience (0-100 scale)
- **Name generator** — Deterministic adjective + noun combinations (~900 combos, e.g. "Cosmic Orbit")
- **CLI commands** — `lean-ctx buddy` with `show`, `stats`, `ascii`, `json` actions; `pet` alias
- **Dashboard Buddy card** — Glasmorphism UI with rarity-dependent gradients/animations, animated XP bar, SVG radial gauges, styled speech bubble, mood indicator
- **API endpoint** — `/api/buddy` serving full `BuddyState` JSON including `ascii_art` and `xp_next_level`

## [2.17.0] — 2026-04-04

### Premium Experience Upgrade — Architecture, Performance & Polish

Major internal refactoring for long-term maintainability, performance improvements for async I/O, unified error handling, and premium polish across CLI, dashboard, and CI pipeline.

#### Architecture
- **server.rs split** — Monolithic `server.rs` (1918 lines) split into 4 focused modules: `tool_defs.rs` (620L), `instructions.rs` (159L), `cloud_sync.rs` (136L), `server.rs` (1001L). Each module has a single responsibility.
- **Centralized error handling** — New `LeanCtxError` enum in `core/error.rs` with `thiserror` derive. `From` impls for `io::Error`, `toml::de::Error`, `serde_json::Error`. `Config::save()` migrated as first consumer.

#### Performance
- **Async I/O for ctx_shell** — `execute_command` wrapped in `tokio::task::spawn_blocking` to prevent blocking the Tokio runtime during shell command execution.

#### CLI
- **Dynamic version** — All hardcoded version strings replaced with `env!("CARGO_PKG_VERSION")`. Version is now single-sourced from `Cargo.toml`.
- **report-issue exit code** — Empty title now exits with status 1 for proper script error detection.
- **Theme migration** — `print_command_box()` migrated from hardcoded ANSI to the `core::theme` system.
- **upgrade → update** — `lean-ctx upgrade` now prints deprecation notice and delegates to `lean-ctx update`.

#### Dashboard
- **Offline fonts** — Removed Google Fonts CDN dependency, switched to system font stacks.
- **Dynamic version** — Version display fetched from `/api/version` instead of hardcoded.
- **Empty state UX** — "No data yet" message links to Getting Started guide.
- **Connection retry** — Auto-retry with clear user message when dashboard API is unavailable.

#### Setup
- **Compact doctor** — New `doctor::run_compact()` provides concise diagnostics during `lean-ctx setup`, reducing noise for new users.

#### Tool Robustness
- **ctx_search** — Reports count of files skipped due to encoding/permission errors.
- **ctx_read** — Warns on unknown mode (falls back to `full`). Shows message when cached content is used after file read failure.
- **ctx_analyze / ctx_benchmark** — `.unwrap()` on `min_by_key` replaced with `if let Some(...)` to prevent potential panics.

#### CI
- **Deduplicated audit** — Removed redundant `cargo audit` job (handled in `security-check.yml`).
- **Release tests** — `cargo test --all-features` now runs before release builds in `release.yml`.

## [2.16.6] — 2026-04-04

### ctx_edit — MCP-native file editing with Windows CRLF support

Agents in Windsurf + Claude Code extension loop when Edit requires unavailable Read.
`ctx_edit` provides search-and-replace as an MCP tool — no native Read/Edit dependency.

#### Added
- **`ctx_edit` MCP tool** — reads, replaces, and writes files in one call. Parameters: `path`, `old_string`, `new_string`, `replace_all`, `create`.

#### Fixed
- **CRLF/LF auto-normalization** — Windows files with `\r\n` now match when agents send `\n` strings (and vice versa). Line endings are preserved.
- **Trailing whitespace tolerance** — retries with trimmed trailing whitespace per line if exact match fails.
- **Edit loop prevention** — instructions say "NEVER loop on Edit failures — use ctx_edit immediately".
- **PREFER over NEVER** — all injected rules use "PREFER lean-ctx tools" instead of "NEVER use native tools".
- **9 unit tests** covering CRLF, LF, trailing whitespace, and combined scenarios.

## [2.15.0] — 2026-04-03

### Scientific Compression Evolution

Six algorithms from information theory, graph theory, and statistical mechanics now power lean-ctx's compression pipeline — all automatic, all local, zero configuration.

### Added
- **Predictive Surprise Scoring** — Replaces static Shannon entropy with BPE cross-entropy. Measures how "surprising" each line is to the LLM's tokenizer. Boilerplate scores low and gets removed; complex logic scores high and stays. 15–30% better filtering than character-level entropy.
- **Spectral Relevance Propagation** — Heat diffusion + PageRank on the project dependency graph. Finds structurally important files even without keyword overlap. Seed files spread relevance along import edges with exponential decay.
- **Boltzmann Context Allocation** — Statistical mechanics-based token budget distribution. Specific tasks concentrate tokens on top files (low temperature); broad tasks spread evenly (high temperature). Automatically selects compression mode per file.
- **Semantic Chunking with Attention Bridges** — Restructures output to counter LLM "Lost in the Middle" attention bias. Promotes task-relevant chunks to high-attention positions, adds structural boundary markers and tail anchors.
- **MMR Deduplication** — Maximum Marginal Relevance removes redundant lines across files using bigram Jaccard similarity. 10–25% less noise in multi-file context loads.
- **BPE-Aligned Token Optimization** — Final-pass string replacements aligned to BPE token boundaries (`function `→`fn `, `" -> "`→`"->"`, lifetime elision). 3–8% additional savings.
- **Auto-Build Graph Index** — `load_or_build()` function automatically builds the project dependency graph on first use. No manual `ctx_graph build` required — the system is fully zero-config.
- **Fish Shell Doctor Check** — `lean-ctx doctor` now detects shell aliases in `~/.config/fish/config.fish` (previously only checked zsh/bash).
- **Codex Hook Refresh on Update** — `lean-ctx update` now refreshes Codex PreToolUse hook scripts alongside Claude, Cursor, and Gemini hooks.

### Changed
- Graph edge resolution now maps Rust module paths back to file paths, enabling correct heat diffusion and PageRank propagation across the codebase.
- Centralized graph index loading across `ctx_preload`, `ctx_overview`, `autonomy`, and `ctx_intent` — eliminates path mismatch bugs between relative and absolute project roots.

### Performance
- **85.7%** session-wide token savings (with CCP) in 30-min coding simulation
- **96%** compression in map/signatures mode with 94% quality preservation
- **99.3%** savings on cache re-reads (13 tokens)
- **95%** git command compression across all patterns
- **12/12** scientific verification checks passed
- **39/39** intensive benchmark tests passed

## [2.14.5] — 2026-04-02

### Changed
- **Internal cleanup** — Removed dead code (`format_type_short`, `instruction_encoding_savings`) and their orphaned test from the protocol module. Simplified cloud and help text messaging. No functional changes.

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
