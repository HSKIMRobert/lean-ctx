# LeanCTX — Project Documentation

**Version:** 2.9.3
**License:** MIT (Open Source)
**Website:** https://leanctx.com
**GitHub:** https://github.com/yvgude/lean-ctx
**GitLab (proprietary):** https://gitlab.pounce.ch/root/lean-ctx
**crates.io:** https://crates.io/crates/lean-ctx
**npm:** https://www.npmjs.com/package/lean-ctx-bin

---

## 1. What is LeanCTX

LeanCTX is a context intelligence engine for AI coding tools. It sits between AI agents (Cursor, Claude Code, Copilot, Windsurf, Zed, etc.) and the development environment, compressing and optimizing all context before it reaches the LLM. This reduces token consumption by 60-99%, resulting in better AI reasoning, longer sessions, and lower costs.

LeanCTX operates on two layers:
- **MCP Server** — 25 intelligent tools that replace standard file reading, searching, and command execution with compressed equivalents
- **Shell Hook** — 90+ command patterns that automatically compress terminal output (git, docker, npm, cargo, kubectl, etc.)

### Key differentiators from competitors (e.g. RTK)
- Session caching: re-reads cost ~13 tokens instead of thousands
- tree-sitter AST parsing for 14 programming languages
- Multiple compression modes (full, map, signatures, aggressive, entropy, diff)
- Cross-session memory (Context Continuity Protocol)
- Multi-agent coordination
- Project knowledge persistence
- Protocol-level output and thinking token optimization (CEP, CRP, TDD)

---

## 2. Architecture

### 2.1 Repository Structure

```
lean-ctx/
├── rust/                    # Core engine (Rust binary)
│   ├── src/
│   │   ├── main.rs          # CLI entry point + MCP server startup
│   │   ├── server.rs        # MCP server (JSON-RPC over stdio)
│   │   ├── shell.rs         # Shell hook execution + compression
│   │   ├── cli.rs           # Shell alias generation (zsh/bash/fish/PowerShell)
│   │   ├── setup.rs         # Auto-detect + configure editors
│   │   ├── hooks.rs         # Agent hook installation (Cursor, Claude Code, etc.)
│   │   ├── doctor.rs        # Diagnostics command
│   │   ├── cloud_client.rs  # Cloud API client (login, sync, contribute)
│   │   ├── lib.rs           # Library exports
│   │   ├── uninstall.rs     # Clean removal
│   │   ├── core/            # Compression engine
│   │   │   ├── cache.rs           # Session cache (file content caching)
│   │   │   ├── compressor.rs      # Core compression logic + lightweight_cleanup
│   │   │   ├── tokens.rs          # Token counting (~4 chars = 1 token)
│   │   │   ├── protocol.rs        # CEP/CRP/TDD instruction generation
│   │   │   ├── entropy.rs         # Shannon entropy filtering
│   │   │   ├── symbol_map.rs      # Identifier shortening (long names → short IDs)
│   │   │   ├── codebook.rs        # TF-IDF cross-file deduplication
│   │   │   ├── adaptive.rs        # Adaptive compression parameters
│   │   │   ├── adaptive_thresholds.rs # Per-language entropy/Jaccard thresholds
│   │   │   ├── mode_predictor.rs  # Auto compression mode selection
│   │   │   ├── task_relevance.rs  # Information Bottleneck filter
│   │   │   ├── feedback.rs        # Compression outcome learning (EWMA)
│   │   │   ├── stats.rs           # Token savings statistics + gain dashboard
│   │   │   ├── session.rs         # Cross-session memory (CCP)
│   │   │   ├── agents.rs          # Multi-agent coordination
│   │   │   ├── knowledge.rs       # Persistent project knowledge
│   │   │   ├── config.rs          # Config loading (~/.lean-ctx/config.toml)
│   │   │   ├── updater.rs         # Self-update from GitHub Releases
│   │   │   ├── preservation.rs    # Edit integrity guarantee
│   │   │   ├── benchmark.rs       # Real compression benchmarks
│   │   │   ├── wrapped.rs         # Shareable savings report cards
│   │   │   ├── slow_log.rs        # Slow command tracking
│   │   │   ├── ctx_response.rs    # Output filler removal
│   │   │   └── patterns/          # 90+ shell compression patterns
│   │   │       ├── mod.rs         # Pattern router
│   │   │       ├── git.rs         # git status/log/diff/push/commit/...
│   │   │       ├── docker.rs      # docker build/ps/images/logs/...
│   │   │       ├── npm.rs         # npm install/test/run/list/...
│   │   │       ├── cargo.rs       # cargo build/test/check/clippy
│   │   │       ├── gh.rs          # GitHub CLI (pr/issue/run)
│   │   │       ├── kubectl.rs     # Kubernetes commands
│   │   │       ├── python.rs      # pip/ruff/poetry/uv
│   │   │       ├── test.rs        # jest/vitest/pytest/go test/playwright
│   │   │       ├── eslint.rs      # ESLint output compression
│   │   │       ├── mypy.rs        # Python type checking
│   │   │       ├── ruby.rs        # rubocop/bundle/rake/rails
│   │   │       ├── terraform.rs   # Terraform plan/apply
│   │   │       └── ... (40+ more)
│   │   ├── tools/           # MCP tool implementations
│   │   │   ├── mod.rs             # Tool registry + CRP mode
│   │   │   ├── ctx_read.rs        # File reading with 6 compression modes
│   │   │   ├── ctx_search.rs      # Regex search with compressed results
│   │   │   ├── ctx_tree.rs        # Directory listing with file counts
│   │   │   ├── ctx_shell.rs       # Shell execution with pattern compression
│   │   │   ├── ctx_multi_read.rs  # Batch file reading
│   │   │   ├── ctx_delta.rs       # Show changes since last read
│   │   │   ├── ctx_smart_read.rs  # Auto-selects best compression mode
│   │   │   ├── ctx_compress.rs    # Context checkpoint compression
│   │   │   ├── ctx_metrics.rs     # Token savings metrics
│   │   │   ├── ctx_graph.rs       # Dependency graph + impact analysis
│   │   │   ├── ctx_overview.rs    # Multi-resolution project overview
│   │   │   ├── ctx_analyze.rs     # Optimal mode recommendation
│   │   │   ├── ctx_benchmark.rs   # Per-file compression benchmark
│   │   │   ├── ctx_dedup.rs       # Duplicate detection
│   │   │   ├── ctx_fill.rs        # Template filling
│   │   │   ├── ctx_intent.rs      # Intent-driven context retrieval
│   │   │   ├── ctx_session.rs     # Session management
│   │   │   ├── ctx_knowledge.rs   # Knowledge CRUD
│   │   │   ├── ctx_agent.rs       # Agent coordination
│   │   │   ├── ctx_discover.rs    # Missed savings discovery
│   │   │   ├── ctx_wrapped.rs     # Savings report generation
│   │   │   ├── ctx_response.rs    # Output token optimization
│   │   │   └── ctx_semantic_search.rs # BM25 semantic code search
│   │   └── dashboard/
│   │       ├── mod.rs             # Dashboard HTTP server
│   │       └── dashboard.html     # Web dashboard (Chart.js)
│   ├── Cargo.toml
│   └── tests/
│       └── integration_tests.rs
├── website/                 # Astro website (leanctx.com) — in .gitignore
│   ├── src/
│   │   ├── pages/           # 20+ pages (docs, features, protocols, etc.)
│   │   ├── components/      # Header, MegaDropdown, SearchModal, etc.
│   │   ├── layouts/         # DocsLayout, BaseLayout
│   │   └── styles/          # global.css (CSS custom properties)
│   ├── astro.config.mjs
│   └── nginx.conf
├── cloud/                   # Cloud backend (proprietary) — in .gitignore
│   ├── src/main.rs          # Axum API server
│   ├── migrations/          # SQLite schema
│   ├── Cargo.toml
│   └── Dockerfile
├── packages/
│   ├── lean-ctx-bin/        # npm wrapper package
│   └── pi-lean-ctx/         # Pi Coding Agent extension
├── aur/
│   ├── lean-ctx/            # AUR source package (git submodule → aur.archlinux.org)
│   └── lean-ctx-bin/        # AUR binary package (git submodule → aur.archlinux.org)
├── homebrew-lean-ctx/       # Separate repo: github.com/yvgude/homebrew-lean-ctx
├── .githooks/
│   └── pre-push             # Blocks proprietary files from GitHub
├── .github/
│   ├── workflows/
│   │   ├── ci.yml           # Rust CI (test, clippy, fmt)
│   │   └── security-check.yml # Guardrail: fails if proprietary code detected
│   └── ISSUE_TEMPLATE/      # Bug report, feature request, compression pattern
├── CONTRIBUTING.md
├── CHANGELOG.md
├── DEPLOY_CHECKLIST.md
├── LICENSE                  # MIT
└── README.md
```

### 2.2 Codebase Metrics

| Metric | Value |
|--------|-------|
| Total Rust lines | ~95,000 |
| Source files | 116 .rs files |
| Shell patterns | 90+ (49 pattern files) |
| MCP tools | 24 |
| Tree-sitter languages | 14 |
| Tests | 215 unit + 5 integration |
| Git tags (releases) | 44 |
| Dependencies | ~40 crates |

---

## 3. Distribution Channels

| Channel | Package | Current Version |
|---------|---------|-----------------|
| **GitHub Releases** | Binary tarballs (macOS, Linux, Windows) | v2.9.3 |
| **crates.io** | `lean-ctx` | 2.9.3 |
| **npm** | `lean-ctx-bin` (binary wrapper) | 2.9.3 |
| **npm** | `pi-lean-ctx` (Pi Coding Agent extension) | 1.0.7 |
| **Homebrew** | `yvgude/lean-ctx/lean-ctx` | 2.9.3 |
| **AUR** | `lean-ctx` (source build) | 2.9.3 |
| **AUR** | `lean-ctx-bin` (pre-built binary) | 2.9.3 |

### Installation Methods

```bash
# Recommended (auto-downloads binary)
curl -fsSL https://raw.githubusercontent.com/yvgude/lean-ctx/main/install.sh | sh

# Via Cargo
cargo install lean-ctx

# Via npm
npx lean-ctx-bin

# Via Homebrew
brew install yvgude/lean-ctx/lean-ctx

# Via AUR (Arch Linux)
yay -S lean-ctx      # source build
yay -S lean-ctx-bin  # pre-built
```

---

## 4. MCP Tools — Complete Reference

### 4.0 All 25 MCP Tools

| # | Tool | Parameters | Description |
|---|------|-----------|-------------|
| 1 | `ctx_read` | `path`, `mode?`, `start_line?`, `fresh?` | Read file with compression. Modes: full, map, signatures, aggressive, entropy, diff, lines:N-M |
| 2 | `ctx_multi_read` | `paths[]`, `mode?` | Batch-read multiple files in one call |
| 3 | `ctx_tree` | `path?`, `depth?`, `show_hidden?` | Directory listing with file counts, respects .gitignore |
| 4 | `ctx_shell` | `command` | Execute shell command, compress output via pattern engine |
| 5 | `ctx_search` | `pattern`, `path?`, `ext?`, `max_results?`, `ignore_gitignore?` | Regex search with compact results, only counts tokens from matched files |
| 6 | `ctx_compress` | `include_signatures?` | Create context checkpoint (compress all cached files) |
| 7 | `ctx_benchmark` | `path`, `action?`, `format?` | Measure compression per mode for a file or entire project |
| 8 | `ctx_analyze` | `path` | Recommend optimal compression mode for a file |
| 9 | `ctx_cache` | `action`, `path?` | Cache operations: status, clear, invalidate(path) |
| 10 | `ctx_metrics` | `top_n?` | Token savings metrics with per-command breakdown |
| 11 | `ctx_smart_read` | `path` | Auto-selects best mode via ModePredictor |
| 12 | `ctx_delta` | `path` | Show changes since last cached read |
| 13 | `ctx_dedup` | `action?`, `paths?` | Cross-file duplicate detection (TF-IDF codebook) |
| 14 | `ctx_fill` | `paths?`, `max_tokens?` | Fill context window up to token budget |
| 15 | `ctx_overview` | `query?`, `project_root?` | Multi-resolution project overview optimized for a task |
| 16 | `ctx_response` | `text` | Compress LLM output text (filler removal) |
| 17 | `ctx_graph` | `action`, `path?`, `project_root?` | Dependency graph: build, related, symbol, impact, status |
| 18 | `ctx_session` | `action`, `value?`, `session_id?` | Session continuity: status, save, load, task, finding, decision |
| 19 | `ctx_knowledge` | `action`, `category?`, `key?`, `value?`, `query?`, `pattern_type?`, `examples?`, `confidence?` | Persistent project knowledge CRUD |
| 20 | `ctx_agent` | `action`, `agent_type?`, `role?`, `message?`, `category?`, `target_agent?`, `status?` | Multi-agent coordination: register, list, post, read, status |
| 21 | `ctx_intent` | `task?`, `project_root?` | Intent-driven context retrieval (Information Bottleneck) |
| 22 | `ctx_wrapped` | `period?` | Generate shareable savings report card |
| 23 | `ctx_semantic_search` | `query`, `path?`, `top_k?`, `action?` | BM25 semantic code search |
| 24 | `ctx_discover` | — | Find missed compression opportunities |

#### Unified Mode (`LEAN_CTX_UNIFIED=1`)

When enabled, the 25 tools are consolidated into 5:

| Unified Tool | Sub-tools |
|-------------|-----------|
| `ctx_read` | read, multi_read, smart_read, delta |
| `ctx_tree` | tree |
| `ctx_shell` | shell |
| `ctx_search` | search, semantic_search |
| `ctx` | compress, metrics, analyze, cache, discover, dedup, fill, intent, response, context, graph, session, knowledge, agent, overview, wrapped, benchmark |

### 4.1 MCP Tool Compression Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `full` | Full content, cached | Files you will edit |
| `map` | Dependency graph + exports + key signatures | Context-only files |
| `signatures` | tree-sitter AST extraction | API surface only |
| `aggressive` | Syntax-stripped (comments, whitespace removed) | Large files |
| `entropy` | Shannon entropy filtering | High-noise files |
| `diff` | Changed lines only | After edits |
| `lines:N-M` | Specific line ranges | Targeted reading |

### 4.2 Session Caching

When a file is read via `ctx_read`, its content is cached in memory. Subsequent reads return a ~13 token reference instead of the full content. This is the single largest token saver because AI agents re-read the same files dozens of times per session.

### 4.3 Shell Hook Patterns

90+ patterns organized by command family:

| Family | Commands | Avg Compression |
|--------|----------|-----------------|
| git | status, log, diff, push, commit, pull, fetch, clone, branch, checkout, merge, stash, tag, reset, remote, blame, cherry-pick | 50-95% |
| docker | build, ps, images, logs, compose, exec, network, volume | 60-90% |
| npm/pnpm | install, test, run, list, outdated, audit | 50-80% |
| cargo | build, test, check, clippy | 60-90% |
| gh | pr, issue, run | 70-90% |
| kubectl | get, logs, describe, apply | 60-85% |
| python | pip, ruff, poetry, uv, pytest, mypy | 50-80% |
| tests | jest, vitest, go test, playwright, rspec | 60-95% |
| linters | eslint, biome, prettier, golangci-lint, rubocop | 50-80% |
| infra | terraform, make, maven, gradle, dotnet, flutter | 40-70% |

### 4.4 Scientific Foundations

| Concept | Implementation | File |
|---------|----------------|------|
| Shannon Entropy | Per-character entropy filtering | `entropy.rs` |
| Jaccard Similarity | Word/n-gram deduplication | `adaptive_thresholds.rs` |
| Kolmogorov Complexity | `K(x) ≈ len(gzip(x)) / len(x)` for compressibility | `mode_predictor.rs` |
| Information Bottleneck | Task-relevance maximization (Tishby et al.) | `task_relevance.rs` |
| BM25 | Semantic code search ranking | `ctx_semantic_search.rs` |
| EWMA | Feedback loop dampening | `feedback.rs` |
| MinHash/LSH | Approximate Jaccard similarity | `codebook.rs` |

### 4.5 Safeguards

Seven mathematical safeguards prevent quality degradation:

1. **Shannon Entropy Floor** — Never compress below language-specific entropy thresholds
2. **Kolmogorov Gate** — Files with K > 0.7 skip aggressive modes (already dense)
3. **Symbol-Map ROI** — Only apply identifier shortening if net savings >= 5%
4. **Token Ratio Bounds** — Compressed output stays within [0.15, 1.0] of original
5. **Edit Integrity** — Files being edited always get `fresh=true` (full re-read)
6. **Feedback Dampening** — EWMA with minimum 5 samples before adjusting thresholds
7. **Benchmark Monotonicity** — Automated test ensures compression never regresses

### 4.6 Compression Pipeline (Data Flow)

When a command or file read enters lean-ctx, it flows through this pipeline:

```
Input (raw output / file content)
  │
  ├─ [Shell Hook Path]
  │   │
  │   ├─ Is command excluded? → pass through unchanged
  │   ├─ Token count < 50? → pass through unchanged
  │   ├─ try_specific_pattern() → route to command-specific pattern
  │   │   ├─ git → git.rs (status/log/diff/push/commit/pull/fetch/clone/branch/checkout/merge/stash/tag/reset/remote/blame/cherry-pick)
  │   │   ├─ gh → gh.rs (pr/issue/run)
  │   │   ├─ docker → docker.rs (build/ps/images/logs/compose/exec/network/volume)
  │   │   ├─ npm/yarn → npm.rs (install/test/run/list/outdated/audit)
  │   │   ├─ cargo → cargo.rs (build/test/check/clippy/install)
  │   │   ├─ kubectl/k → kubectl.rs (get/logs/describe/apply/delete)
  │   │   ├─ terraform → terraform.rs (plan/apply/init/validate)
  │   │   ├─ make → make.rs
  │   │   ├─ mvn/gradle → maven.rs
  │   │   ├─ helm → helm.rs
  │   │   ├─ pnpm → pnpm.rs
  │   │   ├─ bun → bun.rs
  │   │   ├─ deno → deno.rs
  │   │   ├─ pip/pip3 → pip.rs
  │   │   ├─ mypy/dmypy → mypy.rs
  │   │   ├─ pytest → test.rs
  │   │   ├─ ruff → ruff.rs
  │   │   ├─ eslint/biome/stylelint → eslint.rs
  │   │   ├─ prettier → prettier.rs
  │   │   ├─ go/golangci-lint → golang.rs
  │   │   ├─ playwright/cypress → playwright.rs
  │   │   ├─ vitest → test.rs
  │   │   ├─ next/vite → next_build.rs
  │   │   ├─ tsc/typescript → typescript.rs
  │   │   ├─ rubocop/bundle/rake/rails/rspec → ruby.rs
  │   │   ├─ grep/rg → grep.rs
  │   │   ├─ find → find.rs
  │   │   ├─ ls → ls.rs
  │   │   ├─ curl → curl.rs
  │   │   ├─ wget → wget.rs
  │   │   ├─ env/printenv → env_filter.rs
  │   │   ├─ dotnet → dotnet.rs
  │   │   ├─ flutter/dart → flutter.rs
  │   │   ├─ poetry/uv → poetry.rs
  │   │   ├─ aws → aws.rs
  │   │   ├─ psql/pg_* → psql.rs
  │   │   ├─ mysql/mariadb → mysql.rs
  │   │   ├─ prisma → prisma.rs
  │   │   ├─ swift → swift.rs
  │   │   ├─ zig → zig.rs
  │   │   └─ systemd/journalctl → systemd.rs
  │   │
  │   ├─ Fallback chain (if no specific pattern matched):
  │   │   ├─ json_schema::compress() → JSON schema extraction
  │   │   ├─ log_dedup::compress() → Log line deduplication
  │   │   ├─ test::compress() → Generic test output detection
  │   │   └─ lightweight_cleanup() → Whitespace/blank line removal
  │   │
  │   ├─ Safeguard: min 5 output tokens (prevents over-compression)
  │   ├─ Safeguard: compressed must be shorter than original
  │   ├─ If output > 30 lines after cleanup → truncate to first 5 + last 5
  │   └─ Append savings annotation: [lean-ctx: 1000→50 tok, -95%]
  │
  ├─ [MCP Tool Path]
  │   │
  │   ├─ ctx_read(path, mode)
  │   │   ├─ Check cache → if cached, return ~13 token reference
  │   │   ├─ Read file from disk (lossy UTF-8)
  │   │   ├─ Apply compression mode:
  │   │   │   ├─ full → store in cache, assign Fn reference, return content
  │   │   │   ├─ map → extract imports + exports + key function signatures
  │   │   │   ├─ signatures → tree-sitter AST extraction (18 languages)
  │   │   │   ├─ aggressive → aggressive_compress() strips comments, blank lines, indentation
  │   │   │   ├─ entropy → entropy_compress_adaptive() with per-language thresholds
  │   │   │   ├─ diff → show only changed lines since last read
  │   │   │   └─ lines:N-M → return specific line ranges
  │   │   ├─ Apply SymbolMap if ROI >= 5% (identifier shortening)
  │   │   ├─ Append savings annotation
  │   │   └─ Record stats
  │   │
  │   ├─ ctx_search(pattern, path)
  │   │   ├─ Walk directory (respects .gitignore)
  │   │   ├─ Skip binary files
  │   │   ├─ Regex match per line
  │   │   ├─ Count original tokens only for files with matches
  │   │   ├─ Apply SymbolMap if ROI >= 5%
  │   │   └─ Return compact: "N matches in M files:\npath:line content"
  │   │
  │   ├─ ctx_tree(path, depth)
  │   │   ├─ Walk directory (respects .gitignore)
  │   │   ├─ Generate compact tree with file counts per directory
  │   │   └─ Compare against full tree listing for savings metric
  │   │
  │   └─ ctx_shell(command)
  │       ├─ Execute command via system shell
  │       ├─ Route through same pattern pipeline as Shell Hook
  │       └─ Additional: ctx_response filler removal on output
  │
  └─ Output to AI agent
```

### 4.7 Compression Algorithms in Detail

#### aggressive_compress (compressor.rs)

Strips non-essential syntax from source code:
- Removes single-line comments (`//`, `#`, `--`)
- Removes multi-line comments (`/* ... */`)
- Removes blank lines
- Collapses leading whitespace to single space
- Preserves all code logic and string literals

#### lightweight_cleanup (compressor.rs)

Minimal cleanup for unmatched shell output:
- Removes whitespace-only lines
- Collapses consecutive blank lines into one
- Preserves all content structure

#### safeguard_ratio (compressor.rs)

Quality gate applied after compression:
- If compressed output is less than 15% of original → reject, return original
- If compressed output is more than 100% of original → reject, return original
- Prevents both over-compression (losing info) and negative compression (adding overhead)

#### diff_content (compressor.rs)

Computes line-level diff between two versions:
- Outputs only changed lines with `+` (added) and `-` (removed) prefixes
- Used by `ctx_read` diff mode and `ctx_delta`

#### shannon_entropy (entropy.rs)

Calculates per-character Shannon entropy: `H(X) = -Σ p_i · log₂(p_i)`
- Input: text string
- Output: bits per character (typically 3.5-5.0 for code, 4.5+ for natural language)
- Used to identify high-information-density lines

#### token_entropy (entropy.rs)

Like shannon_entropy but operates on whitespace-delimited tokens instead of characters.

#### jaccard_similarity (entropy.rs)

Word-level Jaccard similarity: `J(A, B) = |A ∩ B| / |A ∪ B|`
- Used for deduplication: if two lines have J > threshold, the duplicate is removed
- Threshold is adaptive per language

#### ngram_jaccard (entropy.rs)

N-gram based Jaccard similarity for finer-grained comparison.

#### minhash_signature / minhash_similarity (entropy.rs)

LSH-based approximation of Jaccard similarity for large datasets.

#### kolmogorov_proxy (entropy.rs)

Approximates Kolmogorov complexity: `K(x) ≈ len(gzip(x)) / len(x)`
- Returns ratio 0.0-1.0 (higher = more random/incompressible)
- Used by the Kolmogorov Gate safeguard: files with K > 0.7 skip aggressive modes

#### compressibility_class (entropy.rs)

Classifies content into High/Medium/Low compressibility based on Kolmogorov proxy:
- High: K < 0.3 (very repetitive, highly compressible)
- Medium: K 0.3-0.7 (typical code)
- Low: K > 0.7 (random/encrypted, skip aggressive compression)

#### entropy_compress_adaptive (entropy.rs)

The main entropy compression function:
1. Get adaptive thresholds for the file's language
2. Compute Shannon entropy per line
3. Compute Jaccard similarity between consecutive lines
4. Keep lines above the entropy threshold
5. Remove lines too similar to their neighbors (J > threshold)
6. Always preserve structural lines (function defs, class defs, imports)

#### information_bottleneck_filter (task_relevance.rs)

Based on Tishby et al. (2000). Optimizes: `L_IB = I(X̄; X | Q) − β · I(X̄; Y | Q)`
- Maximizes task relevance while minimizing token usage
- Uses task hints from session context to score relevance
- Applies attention-aware positional weighting (LITM effect: U-curve)

### 4.8 Session Cache Internals (cache.rs)

The session cache is the largest source of token savings:

- **Storage:** In-memory HashMap keyed by file path
- **Entry:** Content, original token count, creation time, hit count, file reference (F1, F2, ...)
- **Cache hit:** Returns `"Fn cached Nt NL"` (~13 tokens) instead of full content
- **Eviction:** LRU-weighted scoring `(age_secs * 0.1) / (hits + 1)`, evicts when total tokens exceed soft limit
- **File references:** Monotonically increasing IDs (F1, F2, F3...) assigned at first read
- **Shared blocks:** Cross-file deduplication via `SharedBlock` entries
- **Invalidation:** `cache.invalidate(path)` removes entry, forcing fresh read
- **Clear:** `cache.clear()` wipes all entries

### 4.9 SymbolMap (symbol_map.rs)

Replaces long identifiers with short IDs to save tokens:

- Scans content for identifiers matching language patterns (camelCase, snake_case, PascalCase)
- Registration criteria: identifier length > short ID length, appears 2+ times
- Short IDs: S0, S1, S2... (2 chars each)
- Appends decode table: `§MAP S0=longIdentifierName S1=anotherLongName`
- ROI gate: only applied if net savings >= 5% (after accounting for table overhead)

### 4.10 Codebook / TF-IDF Deduplication (codebook.rs)

Cross-file deduplication using TF-IDF scoring:

- Builds frequency table from all cached files
- Lines appearing in many files with high TF-IDF score become codebook entries
- Codebook entries get short references (C0, C1...)
- Appends legend mapping references to full content
- Also includes `tfidf_cosine_similarity()` for finding semantically similar files
- `find_semantic_duplicates()` identifies near-duplicate code blocks

### 4.11 Feedback Loop (feedback.rs)

Adaptive learning system that improves compression over time:

- Records `CompressionOutcome` after each compression: language, mode, entropy/jaccard thresholds used, resulting ratio
- Stores outcomes in `~/.lean-ctx/feedback.json`
- Uses EWMA (Exponentially Weighted Moving Average) to smooth threshold updates
- Minimum 5 samples before adjusting thresholds (dampening safeguard)
- Per-language learned thresholds override static defaults
- `format_report()` generates human-readable feedback summary

### 4.12 Adaptive Thresholds (adaptive_thresholds.rs)

Per-language compression parameters:

- Each language gets tuned `CompressionThresholds`:
  - `entropy_threshold`: min Shannon entropy to keep a line
  - `jaccard_threshold`: max similarity before deduplication
  - `preserve_patterns`: regex patterns to always keep (function defs, imports, etc.)
- `thresholds_for_path()`: determines language from file extension
- `adaptive_thresholds()`: combines static defaults with feedback-learned values
- Languages with specific tuning: Rust, TypeScript, JavaScript, Python, Go, Java, C/C++, Ruby, Swift, Kotlin, CSS, HTML, JSON, YAML, TOML, Markdown

### 4.13 Mode Predictor (mode_predictor.rs)

Automatic compression mode selection:

- `FileSignature`: characterizes file by extension, token count, line count
- `ModePredictor`: tracks which mode produced best results for similar files
- `predict_best_mode()`: returns mode with highest historical efficiency for the signature
- Falls back to `predict_from_defaults()` using static rules:
  - Small files (< 200 tokens) → full
  - Config files (json, yaml, toml) → map
  - Source code → signatures or aggressive based on size
- Persists predictions in `~/.lean-ctx/mode_predictor.json`

### 4.14 Token Counting (tokens.rs)

Approximation of LLM tokenization:

```
count_tokens(text) ≈ text.len() / 4
```

Uses a simple heuristic (~4 characters per token) that closely matches GPT/Claude tokenizers for code. Fast enough to run on every compression operation without measurable overhead.

### 4.15 Configuration (config.toml)

Located at `~/.lean-ctx/config.toml`:

```toml
ultra_compact = false           # Enable maximum compression
tee_on_error = false            # Save full output on non-zero exit
checkpoint_interval = 15        # Auto-checkpoint every N tool calls
excluded_commands = []          # Commands to skip compression for
slow_command_threshold_ms = 5000 # Log commands slower than this

[cloud]
contribute_enabled = false      # Share anonymized compression data
```

### 4.16 All Shell Pattern Modules (49 files)

| Module | File | Commands Handled |
|--------|------|-----------------|
| git | git.rs | status, log, diff, add, commit, push, pull, fetch, clone, branch, checkout, switch, merge, stash, tag, reset, remote, blame, cherry-pick |
| gh | gh.rs | pr list/view/create, issue list/view, run list/view, workflow |
| docker | docker.rs | build, ps, images, logs, compose up/down, exec, network ls, volume ls |
| npm | npm.rs | install, test, run, list, outdated, audit |
| pnpm | pnpm.rs | install, test, run, list |
| bun | bun.rs | install, test, run |
| deno | deno.rs | run, test, lint, fmt |
| yarn | (routed via npm.rs) | install, test, run |
| cargo | cargo.rs | build, test, check, clippy, install, fmt |
| kubectl | kubectl.rs | get pods/svc/deploy/nodes, logs, describe, apply, delete |
| helm | helm.rs | install, upgrade, list, status, template |
| terraform | terraform.rs | plan, apply, init, validate, output, state |
| make | make.rs | target execution output |
| maven | maven.rs | mvn/gradle/gradlew build output |
| pip | pip.rs | install, list, outdated |
| ruff | ruff.rs | check, format |
| mypy | mypy.rs | type check output (error grouping, severity counts) |
| eslint | eslint.rs | lint output (error/warning extraction) |
| prettier | prettier.rs | format output |
| golang | golang.rs | go build/test/vet, golangci-lint |
| test | test.rs | Generic: jest, vitest, pytest, go test (pass/fail summary) |
| playwright | playwright.rs | playwright test, cypress run |
| typescript | typescript.rs | tsc output (error extraction) |
| next_build | next_build.rs | next build, vite build |
| ruby | ruby.rs | rubocop, bundle install/update, rake test, rails test, rspec |
| grep | grep.rs | grep, rg (result compaction) |
| find | find.rs | find (path grouping) |
| ls | ls.rs | ls (compact directory listing) |
| curl | curl.rs | curl (response summarization) |
| wget | wget.rs | wget (download progress compression) |
| env_filter | env_filter.rs | env, printenv (sensitive value redaction) |
| dotnet | dotnet.rs | dotnet build/test/run |
| flutter | flutter.rs | flutter build/test, dart analyze |
| poetry | poetry.rs | poetry install/update, uv sync/pip install |
| aws | aws.rs | AWS CLI output |
| psql | psql.rs | PostgreSQL query output |
| mysql | mysql.rs | MySQL/MariaDB query output |
| prisma | prisma.rs | prisma migrate/generate |
| swift | swift.rs | swift build/test |
| zig | zig.rs | zig build/test |
| systemd | systemd.rs | systemctl/journalctl output |
| json_schema | json_schema.rs | Generic JSON → schema extraction |
| log_dedup | log_dedup.rs | Generic log line deduplication |

### 4.17 tree-sitter Languages (14)

| Language | tree-sitter Crate |
|----------|------------------|
| Rust | tree-sitter-rust |
| TypeScript | tree-sitter-typescript |
| JavaScript | tree-sitter-javascript |
| Python | tree-sitter-python |
| Go | tree-sitter-go |
| Java | tree-sitter-java |
| C | tree-sitter-c |
| C++ | tree-sitter-cpp |
| C# | tree-sitter-c-sharp |
| Ruby | tree-sitter-ruby |
| Swift | tree-sitter-swift |
| Kotlin | tree-sitter-kotlin |
| PHP | tree-sitter-php |
| Bash | tree-sitter-bash |

Used in `signatures` mode to extract:
- Function/method definitions (name, parameters, return type)
- Struct/class/interface definitions
- Enum definitions
- Import/use statements
- Public API surface only (skips private internals)

---

## 5. Protocols

### 5.1 CEP (Cognitive Efficiency Protocol)

Instructions injected into the system prompt that guide the LLM to produce shorter, more structured responses. Rules like "ACT FIRST", "DELTA ONLY", "ONE LINE PER ACTION" reduce output token consumption by 50-80%.

### 5.2 CCP (Context Continuity Protocol)

Cross-session memory system. Sessions are saved to `~/.lean-ctx/sessions/` and can be restored in new chats, eliminating cold-start overhead (~400 tokens to restore vs ~50K to rebuild).

### 5.3 TDD (Token Dense Dialect)

Aggressive compression mode using symbols instead of prose. Replaces articles, filler words, and verbose explanations with structured notation.

### 5.4 ctx_response — Output Token Optimization

Compresses LLM response text by removing zero-information filler patterns:

**Standard mode** removes:
- Preamble: "Here's what I found", "Let me explain"
- Hedging: "I think", "I believe", "It seems like"
- Meta-commentary: "That's a great question", "Sure thing"
- Transitions: "Now let's", "Moving on", "Going forward"
- Closings: "Hope this helps", "Let me know if", "Happy to help"
- Acknowledgments: "Understood.", "Got it.", "I see."

**Preserves** lines with information signals: "Note:", "Warning:", "Error:", "However,", "But ", "Important:", "Caution:", "Hint:"

**TDD mode** additionally applies word→symbol shortcuts:
- `function` → `fn`, `configuration` → `cfg`, `implementation` → `impl`
- `returns` → `→`, `successfully` → `✓`, `failed` → `✗`, `warning` → `⚠`
- `approximately` → `≈`, `therefore` → `∴`
- `and` → `&`, `is not` → `≠`, `equals` → `=`

**Safeguards:**
- Responses ≤ 100 tokens: skip compression entirely
- Savings < 20 tokens: return original (compression not worth the processing)

---

## 6. Infrastructure

### 6.1 GitHub (Public — Open Source)

- Repository: `github.com/yvgude/lean-ctx`
- Contains: Engine source, CLI, docs, templates, packages
- CI: GitHub Actions (test, clippy, fmt, security check, CodeQL)
- Releases: Binary tarballs for macOS (aarch64), Linux (x86_64), Windows (x86_64)

### 6.2 GitLab (Private — Proprietary)

- Repository: `gitlab.pounce.ch/root/lean-ctx`
- Contains: Everything from GitHub + `website/`, `cloud/`, `Dockerfile.web`, `.gitlab-ci.yml`
- CI: Docker-in-Docker pipeline builds and deploys website container

### 6.3 Repository Separation (3 Layers)

| Layer | Mechanism | Purpose |
|-------|-----------|---------|
| `.gitignore` | `website/`, `cloud/`, `docker-compose.yml`, `Dockerfile.web`, `.gitlab-ci.yml` | Prevents `git add` |
| Pre-push Hook | `.githooks/pre-push` checks `.github-ignore` list | Blocks push to GitHub |
| CI Guardrail | `.github/workflows/security-check.yml` | Fails on GitHub if proprietary files detected |

### 6.4 Hetzner Server

- Host: `lean-ctx.pounce.ch`
- Runs: Traefik reverse proxy + Docker containers
- Website: `lean-ctx-web` container (Nginx serving Astro static build)
- Cloud API: `lean-ctx-cloud` container (Axum + SQLite)
- Domains: `leanctx.com`, `www.leanctx.com`, `leanctx.tech`, `lean-ctx.pounce.ch`
- TLS: Let's Encrypt via Traefik

### 6.5 Deployment Workflow

```bash
# 1. Website changes (in .gitignore, deploy via GitLab)
git add -f website/ Dockerfile.web .gitlab-ci.yml
git commit -m "deploy: description"
git push origin main              # triggers GitLab pipeline
git rm -r --cached website/ ...   # remove from tracking
git commit -m "chore: remove deploy files"
git push github main              # clean push to GitHub
git push origin main              # clean push to GitLab

# 2. Code changes (normal workflow)
git push github main   # open source
git push origin main   # GitLab mirror
```

---

## 7. Cloud Backend

**Stack:** Rust (Axum), SQLite, JWT, bcrypt

### API Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/api/auth/register` | POST | — | Register user (email + API key) |
| `/api/auth/me` | GET | API key | Get user profile |
| `/api/stats` | POST | API key | Upload token savings stats |
| `/api/stats` | GET | API key | Get user stats |
| `/api/stats/summary` | GET | API key | Aggregated stats summary |
| `/api/contribute` | POST | API key | Submit anonymized compression data |
| `/api/contribute/stats` | GET | — | Public aggregate contribution stats |
| `/api/sync/knowledge` | POST | API key | Push project knowledge to cloud |
| `/api/sync/knowledge` | GET | API key | Pull team knowledge from cloud |
| `/api/checkout` | POST | API key | Create Stripe checkout session |
| `/api/webhooks/stripe` | POST | Stripe sig | Handle subscription events |

### Database Schema (SQLite)

Tables: `users`, `stats`, `teams`, `shared_knowledge`, `collective_data`

---

## 8. Website

**Stack:** Astro, Tailwind CSS, Pagefind (search), Chart.js, Nginx

### Pages

| Page | Path | Description |
|------|------|-------------|
| Home | `/` | Hero, features overview, social proof |
| How It Works | `/how-it-works` | Visual explanation of compression layers |
| Context Server | `/mcp-server` | MCP tool documentation |
| Shell Hook | `/shell-hook` | Pattern documentation |
| Protocols | `/protocols` | CEP, CCP, TDD overview |
| CLI Reference | `/cli` | Terminal command reference |
| Compatibility | `/compatibility` | Supported AI tools matrix |
| Benchmark | `/benchmark` | Run benchmarks on your codebase |
| Getting Started | `/docs/getting-started` | Installation guide with prompt generator |
| Configuration | `/docs/configuration` | config.toml reference |
| Analytics | `/docs/analytics` | Gain, wrapped, dashboard guide |
| Tool API | `/docs/tools` | All 25 MCP tools documented |
| Tree-sitter | `/docs/tree-sitter` | AST engine documentation |
| Dashboard | `/dashboard/` | Web analytics dashboard |

---

## 9. Supported AI Tools

LeanCTX integrates with 18+ AI coding tools:

| Tool | Integration | Setup |
|------|-------------|-------|
| Cursor | MCP server (`.cursor/mcp.json`) | `lean-ctx init --agent cursor` |
| Claude Code | MCP server (`.claude/claude_code_config.json`) | `lean-ctx init --agent claude-code` |
| VS Code (Copilot) | MCP server (`.vscode/mcp.json`) | `lean-ctx init --agent vscode` |
| Windsurf | MCP server (`.windsurf/mcp.json`) | `lean-ctx init --agent windsurf` |
| Zed | MCP server (`settings.json`) | `lean-ctx init --agent zed` |
| OpenCode | MCP server (`.opencode/config.json`) | `lean-ctx init --agent opencode` |
| Continue.dev | MCP server (`config.yaml`) | `lean-ctx init --agent continue` |
| Cline / Roo | MCP server (`.cline/mcp.json`) | `lean-ctx init --agent cline` |
| Crush | MCP server (`crush.json`) | `lean-ctx init --agent crush` |
| Gemini CLI | Shell hook | `lean-ctx init --global` |
| Aider | Shell hook | `lean-ctx init --global` |
| Pi Coding Agent | Extension package | `npm install pi-lean-ctx` |
| Any MCP client | MCP server (stdio) | Manual config |

---

## 10. CLI Commands

### Core

| Command | Description |
|---------|-------------|
| `lean-ctx` | Start MCP server (stdio mode, JSON-RPC over stdin/stdout) |
| `lean-ctx -c "command"` | Execute command with compressed output (shell hook) |
| `lean-ctx exec "command"` | Same as `-c` |
| `lean-ctx shell` | Interactive shell with compression |
| `lean-ctx read <file>` | Read file with compression (uses map mode by default) |
| `lean-ctx --version` | Print version |
| `lean-ctx --help` | Print help |

### Analytics

| Command | Description |
|---------|-------------|
| `lean-ctx gain` | Visual terminal dashboard with savings summary |
| `lean-ctx gain --live` | Auto-refreshing dashboard (updates every 5s) |
| `lean-ctx gain --graph` | 30-day ASCII chart of daily savings |
| `lean-ctx gain --daily` | Day-by-day table with token counts and USD values |
| `lean-ctx gain --json` | Raw JSON export of all stats |
| `lean-ctx dashboard` | Web dashboard (opens localhost:3333 in browser, Chart.js) |
| `lean-ctx wrapped` | Shareable savings report card (like Spotify Wrapped) |
| `lean-ctx cep` | CEP Intelligence report (cache hits, mode diversity, complexity) |

### Setup

| Command | Description |
|---------|-------------|
| `lean-ctx setup` | One-command setup (shell aliases + auto-detect editors) |
| `lean-ctx init --global` | Install shell aliases (zsh/bash/fish/PowerShell) |
| `lean-ctx init --agent <name>` | Configure MCP for a specific editor |
| `lean-ctx init --agent pi` | Install Pi Coding Agent extension |
| `lean-ctx doctor` | Full diagnostics (shell, editors, permissions, versions) |
| `lean-ctx update` | Self-update from GitHub Releases (Windows: deferred update) |
| `lean-ctx uninstall` | Clean removal (binary, shell configs, ~/.lean-ctx) |
| `lean-ctx config` | Show current configuration |

### Cloud

| Command | Description |
|---------|-------------|
| `lean-ctx login <email>` | Register/login (creates API key) |
| `lean-ctx sync` | Upload token savings stats to cloud |
| `lean-ctx contribute` | Share anonymized compression data for Collective Intelligence |
| `lean-ctx upgrade` | Subscribe to Pro ($9/mo, opens browser for Stripe checkout) |
| `lean-ctx team push` | Push local project knowledge to team cloud |
| `lean-ctx team pull` | Pull shared team knowledge from cloud |

### Shell Aliases (installed by `lean-ctx init --global`)

After setup, these shell functions are available system-wide:

| Alias | Description |
|-------|-------------|
| `lean-ctx-on` | Enable shell hook (all commands pass through lean-ctx) |
| `lean-ctx-off` | Disable shell hook (direct output, no compression) |
| `lean-ctx-status` | Show whether hook is active + current version |

Implementation: modifies `preexec`/`precmd` (zsh), `trap DEBUG` (bash), or `fish_preexec` (fish) to intercept commands and pipe them through `lean-ctx exec`.

### How `lean-ctx exec` Works

```
1. User types command (e.g. "git status")
2. Shell hook intercepts via preexec
3. lean-ctx exec "git status" is called instead
4. lean-ctx:
   a. Spawns the actual command as a child process
   b. Captures stdout + stderr
   c. If exit code != 0 and tee_on_error enabled: save raw output
   d. Route output through compress_output():
      - Check excluded_commands list
      - Check custom_aliases (map user aliases to known commands)
      - Try specific pattern (git.rs, docker.rs, etc.)
      - Try fallback chain (json_schema, log_dedup, test, lightweight_cleanup)
      - Apply safeguard ratio check
   e. Record stats (original tokens, compressed tokens, command, timestamp)
   f. Output compressed result to stdout
   g. If command took > slow_command_threshold_ms: log to slow log
5. LLM receives compressed output
```

---

## 10b. Cost Model

The `lean-ctx gain` dashboard calculates monetary savings using these constants:

| Parameter | Value | Basis |
|-----------|-------|-------|
| Input price | $2.50/M tokens | Claude Sonnet 4 average |
| Output price | $10.00/M tokens | Claude Sonnet 4 average |
| Avg verbose output/call | 450 tokens | Without CEP/CRP protocols |
| Avg concise output/call | 120 tokens | With CEP/CRP protocols |

**Formula:**
```
Input savings = (total_input_tokens - compressed_output_tokens) × $2.50/M
Output savings = (verbose_output - concise_output) × total_calls × $10.00/M
Total saved = Input savings + Output savings
```

The output savings account for CEP/CRP protocol effects: LLMs produce shorter responses when instructed with structured protocols. The ratio `120/450 ≈ 73%` output reduction is based on empirical measurements.

---

## 10c. Storage Locations

All lean-ctx data is stored under `~/.lean-ctx/`:

| Path | Content |
|------|---------|
| `~/.lean-ctx/config.toml` | User configuration |
| `~/.lean-ctx/stats.json` | Token savings statistics (per command, per day) |
| `~/.lean-ctx/feedback.json` | Compression outcome feedback (per language thresholds) |
| `~/.lean-ctx/mode_predictor.json` | Learned optimal modes per file signature |
| `~/.lean-ctx/sessions/` | Cross-session memory files |
| `~/.lean-ctx/knowledge/` | Persistent project knowledge |
| `~/.lean-ctx/agents/` | Multi-agent coordination state |
| `~/.lean-ctx/slow_log.json` | Commands exceeding threshold |
| `~/.lean-ctx/wrapped/` | Generated report cards |
| `~/.lean-ctx/credentials.json` | Cloud API key (created by `lean-ctx login`) |
| `~/.lean-ctx/adaptive/` | Adaptive Intelligence model data (Pro) |

---

## 11. Release Process

Full checklist for every release (see `DEPLOY_CHECKLIST.md`):

1. Bump version in: `Cargo.toml`, `main.rs` (3x), `server.rs`, `shell.rs`, `stats.rs`, `cli.rs`, `dashboard.html`, `packages/lean-ctx-bin/package.json`, `README.md`
2. Bump `pi-lean-ctx` version in `packages/pi-lean-ctx/package.json`
3. Update `CHANGELOG.md`
4. Run: `cargo test && cargo clippy && cargo fmt --check`
5. `cargo build --release`
6. `git add -A && git commit && git tag vX.Y.Z`
7. `git push github main --tags`
8. Create GitHub Release with binary tarballs
9. `cargo publish` (crates.io)
10. `npm publish` (lean-ctx-bin + pi-lean-ctx)
11. Update AUR PKGBUILDs + .SRCINFOs, push to `aur.archlinux.org`
12. Download GitHub tarball, compute sha256, update Homebrew formula, push to `homebrew-lean-ctx`
13. Test `brew install yvgude/lean-ctx/lean-ctx` from scratch
14. Push to GitLab: `git push origin main`
15. Deploy website if needed (force-add, push, remove from tracking)

---

## 12. Known Issues and Decisions

### Resolved (v2.9.x)

| Issue | Version | Fix |
|-------|---------|-----|
| Homebrew sha256 mismatch | v2.9.2 | GitHub regenerates tarballs; always verify hash after push |
| git push loses pipeline URLs | v2.9.2 | Preserve `remote:` lines with URLs |
| git commit loses hook output | v2.9.2 | Show pre-commit output before commit summary |
| ctx_search inflated savings | v2.9.3 | Only count tokens of files with matches |
| ctx_tree inflated savings | v2.9.3 | Raw comparison now respects .gitignore |
| Windows update "Access denied" | v2.9.0 | Deferred update via background .bat script |
| Config overwrite (#29) | v2.9.0 | Merge into existing JSON instead of overwriting |
| UTF-8 file reading (#28) | v2.9.0 | Lossy UTF-8 decoding |

### Architectural Decisions

- **Single binary** — No runtime dependencies, easy distribution
- **Rust** — Performance, memory safety, cross-compilation
- **MCP over stdio** — Standard protocol, works with all MCP clients
- **SQLite for cloud** — Simple, no external database needed
- **Astro for website** — Static site generation, fast, SEO-friendly
- **Open-core model** — Engine is MIT open source, cloud services are proprietary

---

## 13. Community and Metrics

| Metric | Value |
|--------|-------|
| GitHub stars | Growing (check live) |
| Total downloads (all channels) | 2,500+ |
| Discord members | Active community |
| Data sharing contributors | 20 entries |
| GitHub issues (total) | 32 (all resolved) |
| Contributors | 1 main + community bug reports |

---

## 14. Monetization Status

### Current (Free)

The entire engine is free and open source (MIT). All compression, caching, protocols, and analytics work locally without any cloud dependency.

### Planned (Pro — $9/month)

Currently under evaluation. Potential Pro features:
- Adaptive compression models trained on community data
- Cloud dashboard and cross-device sync
- Team knowledge sharing
- Custom compression patterns
- Priority support

The Pro navigation link is currently hidden on the website pending strategic finalization.

### Cloud API

The cloud backend is deployed and operational at `api.leanctx.com`. It handles user registration, stats collection, anonymized data contribution, and is prepared for Stripe checkout integration.
