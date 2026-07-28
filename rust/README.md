README.md [811L]
# lean-ctx
**Context Engineering for AI Agents with CCP + TDD. Shell Hook + MCP Server. 82 MCP tools, 10 read modes, 95+ shell patterns, cross-session memory (CCP), LITM-aware positioning, tree-sitter AST for 26 languages. Single Rust binary.**
... [lean-ctx: omitted 7 lines]
[Website](https://leanctx.com) · [Install](#installation) · [Quick Start](#quick-start) · [CLI Reference](#cli-commands) · [MCP Tools](#79-mcp-tools) · [Changelog](CHANGELOG.md) · [vs RTK](#lean-ctx-vs-rtk) · [Discord](https://discord.gg/pTHkG9Hew9)
... [lean-ctx: omitted 1 lines]
lean-ctx reduces LLM token consumption by **up to 99%** through two complementary strategies in a single binary:
... [lean-ctx: omitted 1 lines]
2. **MCP Server** — 80 tools for cached file reads, adaptive mode selection, incremental deltas, dependency maps, intent detection, cross-file dedup, project graph, cross-session memory (CCP), multi-agent coordination, semantic caching, and session metrics. Works with Cursor, GitHub Copilot, Claude Code, CodeBuddy, Windsurf, OpenAI Codex, Google Antigravity, OpenCode, and any MCP-compatible editor.
3. **AI Tool Hooks** — One-command integration for Claude Code, CodeBuddy, Cursor, Gemini CLI, Codex, Crush, Windsurf, and Cline via `lean-ctx init --agent <tool>`.
## Token Savings (Typical Cursor/Claude Code Session)
| Operation | Frequency | Standard | lean-ctx | Savings |
... [lean-ctx: omitted 4 lines]
| git status/log/diff | 10x | 8,000 | 2,400 | **-70%** |
... [lean-ctx: omitted 1 lines]
| cargo/npm build | 5x | 5,000 | 1,000 | **-80%** |
... [lean-ctx: omitted 2 lines]
| docker ps/build | 3x | 900 | 180 | **-80%** |
... [lean-ctx: omitted 1 lines]
> Estimates based on medium-sized TypeScript/Rust projects. MCP cache hits reduce re-reads to ~13 tokens each.
## Installation
### Homebrew (macOS / Linux)
```bash
brew tap yvgude/lean-ctx
brew install lean-ctx
```
### Arch Linux (AUR)
```bash
yay -S lean-ctx        # builds from source (crates.io)
# or
yay -S lean-ctx-bin    # pre-built binary (GitHub Releases)
```
### Cargo
```bash
cargo install lean-ctx
```
### Build from Source
```bash
git clone https://github.com/yvgude/lean-ctx.git
cd lean-ctx/rust
cargo build --release
cp target/release/lean-ctx ~/.local/bin/
```
... [lean-ctx: omitted 8 lines]
### Verify Installation
```bash
lean-ctx --version   # Should show "lean-ctx 3.6.10"
lean-ctx gain        # Should show token savings stats
```
## Token Dense Dialect (TDD)
lean-ctx introduces **TDD mode** — enabled by default. TDD compresses LLM communication using mathematical symbols and short identifiers:
... [lean-ctx: omitted 9 lines]
- Signatures use compact notation: `λ+handle(⊕,path:s)→s` instead of `fn pub async handle(&self, path: String) -> String`
... [lean-ctx: omitted 2 lines]
**Result**: 8-25% additional savings on top of existing compression.
Configure with `LEAN_CTX_CRP_MODE`:
- `tdd` (default) — Maximum compression with symbol shorthand
- `compact` — Moderate: skip filler words, use abbreviations
- `off` — Standard output, no CRP instructions
## Quick Start
```bash
# 1. Install
cargo install lean-ctx

# 2. Set up shell hook (auto-installs aliases)
lean-ctx init --global

# 3. Configure your editor (example: Cursor)
# Add to ~/.cursor/mcp.json:
# { "mcpServers": { "lean-ctx": { "command": "lean-ctx" } } }

# 4. Restart your shell + editor, then test
git status       # Automatically compressed via shell hook
lean-ctx gain    # Check your savings
```
... [lean-ctx: omitted 1 lines]
## How It Works
```
  Without lean-ctx:                              With lean-ctx:

  LLM --"read auth.ts"--> Editor --> File        LLM --"ctx_read auth.ts"--> lean-ctx --> File
    ^                                  |           ^                           |            |
    |      ~2,000 tokens (full file)   |           |   ~13 tokens (cached)     | cache+hash |
    +----------------------------------+           +------ (compressed) -------+------------+

  LLM --"git status"-->  Shell  -->  git         LLM --"git status"-->  lean-ctx  -->  git
    ^                                 |            ^                       |              |
    |     ~800 tokens (raw output)    |            |   ~150 tokens         | compress     |
    +---------------------------------+            +------ (filtered) -----+--------------+
```
... [lean-ctx: omitted 5 lines]
## CLI Commands
### Shell Hook
```bash
lean-ctx -c "git status"       # Execute + compress output
lean-ctx exec "cargo build"    # Same as -c
lean-ctx shell                 # Interactive REPL with compression
```
### File Operations
```bash
lean-ctx read file.rs                    # Full content (with structured header)
lean-ctx read file.rs -m map             # Dependency graph + API signatures (~10% tokens)
lean-ctx read file.rs -m signatures      # Function/class signatures only (~15% tokens)
lean-ctx read file.rs -m aggressive      # Syntax-stripped content (~40% tokens)
lean-ctx read file.rs -m entropy         # Shannon entropy filtered (~30% tokens)
lean-ctx read file.rs -m "lines:10-50,80-90"  # Specific line ranges (comma-separated)
lean-ctx diff file1.rs file2.rs          # Compressed file diff
lean-ctx grep "pattern" src/             # Grouped search results
lean-ctx find "*.rs" src/                # Compact find results
lean-ctx ls src/                         # Token-optimized directory listing
lean-ctx deps .                          # Project dependencies summary
```
### Context Packages
```bash
lean-ctx pack create --name my-pkg          # Bundle Knowledge + Graph + Session + Gotchas
lean-ctx pack list                          # List installed packages
lean-ctx pack info my-pkg                   # Detailed view (stats, integrity, provenance)
lean-ctx pack export my-pkg -o my.ctxpkg   # Export to portable .ctxpkg file
lean-ctx pack import my.ctxpkg --apply     # Import and apply to current project
lean-ctx pack install my-pkg                # Apply package (merge knowledge, import graph)
lean-ctx pack auto-load my-pkg              # Auto-load on ctx_overview session start
lean-ctx pack remove my-pkg                 # Remove from local registry
lean-ctx pack --pr                          # PR context pack (unchanged)
```
### Setup & Analytics
```bash
lean-ctx init --global         # Install 23 shell aliases (.zshrc/.bashrc/.config/fish)
lean-ctx init --agent claude    # Install Claude Code PreToolUse hook
lean-ctx init --agent codebuddy # Install CodeBuddy PreToolUse hook
lean-ctx init --agent cursor   # Install Cursor hooks.json
lean-ctx init --agent gemini   # Install Gemini CLI BeforeTool hook
lean-ctx init --agent codex    # Install Codex AGENTS.md + compatible hooks
lean-ctx init --agent windsurf # Install .windsurfrules
lean-ctx init --agent cline    # Install .clinerules
lean-ctx init --agent crush    # Install Crush MCP config
lean-ctx gain                  # Persistent token savings (CLI)
lean-ctx gain --graph          # ASCII chart of last 30 days
lean-ctx gain --daily          # Day-by-day breakdown
lean-ctx gain --json           # Raw JSON export of all stats
lean-ctx dashboard             # Web dashboard at localhost:3333
lean-ctx dashboard --port=8080 # Custom port
lean-ctx discover              # Find uncompressed commands in shell history
lean-ctx session               # Show adoption statistics
lean-ctx config                # Show configuration (~/.lean-ctx/config.toml)
lean-ctx config init           # Create default config file
lean-ctx doctor                # Diagnostics: PATH, config, aliases, MCP, ports
lean-ctx wrapped               # Shareable savings report (CCP)
lean-ctx wrapped --week        # Weekly savings report
lean-ctx sessions list         # List CCP sessions
lean-ctx sessions show <id>    # Show session details
lean-ctx sessions delete <id>  # Delete one session
lean-ctx sessions cleanup      # Remove old sessions
lean-ctx benchmark run         # Real project benchmark (terminal)
lean-ctx benchmark run --json  # Machine-readable JSON output
lean-ctx benchmark report      # Shareable Markdown report
lean-ctx --version             # Show version
lean-ctx --help                # Full help
```
### MCP Server
```bash
lean-ctx                       # Start MCP server (stdio) — used by editors
```
## Shell Hook Patterns (95+)
The shell hook applies pattern-based compression for 95+ commands across 34 categories:
... [lean-ctx: omitted 2 lines]
| **Git** (19) | status, log, diff, add, commit, push, pull, fetch, clone, branch, checkout, switch, merge, stash, tag, reset, remote, blame, cherry-pick | -70-95% |
... [lean-ctx: omitted 1 lines]
| **npm/pnpm/yarn** (6) | install, test, run, list, outdated, audit | -70-90% |
... [lean-ctx: omitted 1 lines]
| **GitHub CLI** (9) | pr list/view/create/merge, issue list/view/create, run list/view | -60-80% |
| **Kubernetes** (8) | get pods/services/deployments, logs, describe, apply, delete, exec, top, rollout | -60-85% |
... [lean-ctx: omitted 2 lines]
| **Linters** (4) | eslint, biome, prettier, stylelint | -60-70% |
... [lean-ctx: omitted 2 lines]
| **Terraform** | init, plan, apply, destroy, validate, fmt, state, import, workspace | -60-85% |
| **Make** | make targets, parallel jobs (`-j`), dry-run (`-n`) | -60-80% |
... [lean-ctx: omitted 5 lines]
| **Databases** (2) | psql, mysql/mariadb | -50-80% |
| **Prisma** (6) | generate, migrate, db push/pull, format, validate | -70-85% |
... [lean-ctx: omitted 10 lines]
| **systemd** (2) | systemctl, journalctl | -50-80% |
... [lean-ctx: omitted 1 lines]
| **Data** (3) | env (filtered), JSON schema extraction, log deduplication | -50-80% |
Unrecognized commands get generic compression: ANSI stripping, empty line removal, and long output truncation.
### 23 Auto-Rewritten Aliases
After `lean-ctx init --global`, these commands are transparently compressed:
```
git, npm, pnpm, yarn, cargo, docker, docker-compose, kubectl, k,
gh, pip, pip3, ruff, go, golangci-lint, eslint, prettier, tsc,
ls, find, grep, curl, wget
```
... [lean-ctx: omitted 1 lines]
## Examples
**Directory listing:**
```
# ls -la src/ (22 lines, ~239 tokens)      # lean-ctx -c "ls -la src/" (8 lines, ~46 tokens)
total 96                                     core/
drwxr-xr-x  4 user staff  128 ...           tools/
drwxr-xr-x  11 user staff 352 ...           cli.rs  9.0K
-rw-r--r--  1 user staff  9182 ...           main.rs  4.0K
-rw-r--r--  1 user staff  4096 ...           server.rs  11.9K
...                                          shell.rs  5.2K
                                             4 files, 2 dirs
                                             [lean-ctx: 239→46 tok, -81%]
```
... [lean-ctx: omitted 29 lines]
```
$ lean-ctx gain

  ◆ lean-ctx  Token Savings Dashboard
  ────────────────────────────────────────────────────────

   1.7M          76.8%         520          $4.25
   tokens saved   compression    commands       USD saved

  Since 2026-03-23 (2 days)  ▁█

  Top Commands
  ────────────────────────────────────────────────────────
  curl                48x  ████████████████████ 728.1K  97%
  git commit          34x  ██████████▎          375.2K  50%
  git rm               7x  ████████▌            313.4K  100%
  ctx_read           103x  █▌                    59.1K  38%
  cat                 15x  ▊                     29.3K  92%
    ... +33 more commands

  Recent Days
  ────────────────────────────────────────────────────────
  03-23    101 cmds      9.4K saved   46.0%
  03-24    419 cmds      1.7M saved   77.0%

  lean-ctx v3.6.10  |  leanctx.com  |  lean-ctx dashboard
```
## 82+ MCP Tools
When configured as an MCP server, lean-ctx provides 80 tools that replace or augment your editor's built-in tools:
### Core Tools
| Tool | Purpose | Savings |
... [lean-ctx: omitted 3 lines]
| `ctx_tree` | Directory listings (ls, find, Glob) | 34-60% |
| `ctx_shell` | Shell commands with 95+ compression patterns | 60-90% |
... [lean-ctx: omitted 1 lines]
| `ctx_compress` | Context checkpoint for long conversations | 90-99% |
### Intelligence Tools
| Tool | Purpose |
... [lean-ctx: omitted 1 lines]
| `ctx_smart_read` | Adaptive mode selection — automatically picks full/map/signatures/diff based on file type, size, and cache state |
... [lean-ctx: omitted 1 lines]
| `ctx_dedup` | Cross-file deduplication — finds shared imports and boilerplate across cached files |
| `ctx_fill` | Priority-based context filling — maximizes information within a token budget |
| `ctx_intent` | Semantic intent detection — classifies queries and auto-loads relevant files |
| `ctx_response` | Response compression — removes filler content, applies TDD shortcuts |
... [lean-ctx: omitted 1 lines]
| `ctx_graph` | Project intelligence graph — dependency analysis and related file discovery |
... [lean-ctx: omitted 5 lines]
### Memory & Multi-Agent Tools
| Tool | Purpose |
... [lean-ctx: omitted 1 lines]
| `ctx_session` | Cross-session memory — persist task, findings, decisions, files across chats and context compactions |
| `ctx_knowledge` | Persistent project knowledge — remember, recall, export, import, remove, search, timeline, relations |
| `ctx_agent` | Multi-agent coordination — register, post/read scratchpad, handoff tasks, sync status |
... [lean-ctx: omitted 2 lines]
### Analysis Tools
| Tool | Purpose |
... [lean-ctx: omitted 1 lines]
| `ctx_benchmark` | Single-file or project-wide benchmark with preservation scores |
| `ctx_metrics` | Session statistics with USD cost estimates ($2.50/1M) |
... [lean-ctx: omitted 1 lines]
| `ctx_compare` | Preview compression — original vs the bytes lean-ctx would emit, with token counts + line diff (read-only) |
... [lean-ctx: omitted 1 lines]
### ctx_read Modes
| Mode | When to use | Token cost |
... [lean-ctx: omitted 2 lines]
| `map` | Understanding a file without reading it — dependency graph + exports + API | ~5-15% |
... [lean-ctx: omitted 2 lines]
| `aggressive` | Large files with boilerplate | ~30-50% |
| `entropy` | Files with repetitive patterns (Shannon + Jaccard filtering) | ~20-40% |
| `lines:N-M` | Only specific line ranges (e.g. `lines:10-50,80-90`) | proportional to selected lines |
### Cache Safety
The session cache auto-clears after 5 minutes of inactivity (configurable via `LEAN_CTX_CACHE_TTL`). This handles new chats, context compaction, and session resets server-side without relying on the LLM.
... [lean-ctx: omitted 3 lines]
- Call `ctx_cache(action: "invalidate", path: "...")` to reset a single file
### Context Continuity Protocol (CCP)
New in v2.0.0: CCP provides cross-session memory that persists across chats, context compactions, and IDE restarts. The session state captures your current task, findings, decisions, and files touched — automatically loaded into every new conversation.
... [lean-ctx: omitted 3 lines]
- Uses LITM-aware positioning: critical context placed at the beginning and end of the LLM's context window (where attention is highest), avoiding the "Lost in the Middle" degradation zone
... [lean-ctx: omitted 3 lines]
```bash
lean-ctx sessions list              # List all sessions
lean-ctx sessions show <id>         # Show session details
lean-ctx sessions delete <id>       # Delete one session
lean-ctx sessions cleanup           # Remove old sessions
lean-ctx wrapped                    # Shareable savings report
lean-ctx wrapped --week             # Weekly report
lean-ctx benchmark run              # Real project benchmark
lean-ctx benchmark report           # Shareable Markdown report
```
... [lean-ctx: omitted 1 lines]
```json
{"tool": "ctx_session", "arguments": {"action": "status"}}
{"tool": "ctx_session", "arguments": {"action": "task", "value": "Implement auth module"}}
{"tool": "ctx_session", "arguments": {"action": "finding", "value": "Auth uses JWT with RS256"}}
{"tool": "ctx_session", "arguments": {"action": "decision", "value": "Use middleware pattern for auth"}}
{"tool": "ctx_gain", "arguments": {"action": "wrapped"}}
```
## Editor Configuration
### Cursor
Add to `~/.cursor/mcp.json`:
```json
{
  "mcpServers": {
    "lean-ctx": {
      "command": "lean-ctx"
    }
  }
}
```
### GitHub Copilot
Add `.github/mcp.json` to your project (Copilot CLI):
```json
{
  "mcpServers": {
    "lean-ctx": {
      "command": "lean-ctx"
    }
  }
}
```
... [lean-ctx: omitted 11 lines]
### Claude Code
```bash
claude mcp add lean-ctx lean-ctx
```
### Windsurf
Add to `~/.codeium/windsurf/mcp_config.json`:
```json
{
  "mcpServers": {
    "lean-ctx": {
      "command": "lean-ctx"
    }
  }
}
```
> **Troubleshooting:** If Windsurf detects the server but tools don't load, use the **full path** to the binary (e.g., `/Users/you/.cargo/bin/lean-ctx` or `/usr/local/bin/lean-ctx`). Windsurf spawns MCP servers with a minimal PATH that may not include `~/.cargo/bin`. Find your path with `which lean-ctx`.
### OpenAI Codex
Add to `~/.codex/config.toml`:
```toml
[mcp_servers.lean-ctx]
command = "lean-ctx"
args = []
```
... [lean-ctx: omitted 6 lines]
- `~/.codex/AGENTS.md` + `~/.codex/LEAN-CTX.md`
- a `PreToolUse` hook that transparently rewrites rewritable Bash commands to `lean-ctx -c "<command>"` (allowed + `updatedInput`), so shell output is compressed with zero agent effort
- a `SessionStart` hook that teaches Codex the raw escape hatch — `lean-ctx raw "<command>"` for the full, exact output — so it never re-reads a compressed view in small chunks
### Google Antigravity
Add to `~/.gemini/antigravity/mcp_config.json`:
```json
{
  "mcpServers": {
    "lean-ctx": {
      "command": "lean-ctx"
    }
  }
}
```
### OpenCode
Add to `~/.config/opencode/opencode.json` (global) or `opencode.json` (project):
```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "lean-ctx": {
      "type": "local",
      "command": ["lean-ctx"],
      "enabled": true
    }
  }
}
```
### OpenClaw
OpenClaw supports MCP servers natively. Run the init command to configure lean-ctx as an MCP server and install skills:
```bash
lean-ctx init --agent openclaw
```
This writes the MCP server entry to `~/.openclaw/openclaw.json` under `mcp.servers`, installs global rules, and copies the LeanCTX skill to `~/.openclaw/skills/lean-ctx/`. Restart OpenClaw to activate.
... [lean-ctx: omitted 1 lines]
### Cursor Terminal Profile
Add a lean-ctx terminal profile for automatic shell hook in Cursor:
```json
{
  "terminal.integrated.profiles.osx": {
    "lean-ctx": {
      "path": "lean-ctx",
      "args": ["shell"],
      "icon": "terminal"
    }
  }
}
```
### Cursor Rule (Optional)
For maximum token savings, add a Cursor rule to your project:
```bash
cp rust/examples/lean-ctx.mdc .cursor/rules/lean-ctx.mdc
```
... [lean-ctx: omitted 1 lines]
## Configuration
### Shell Hook Setup
```bash
lean-ctx init --global
```
This adds 23 aliases (git, npm, pnpm, yarn, cargo, docker, kubectl, gh, pip, ruff, go, golangci-lint, eslint, prettier, tsc, ls, find, grep, curl, wget, and more) to your `.zshrc` / `.bashrc` / `config.fish`.
... [lean-ctx: omitted 1 lines]
```bash
alias git='lean-ctx -c git'
alias npm='lean-ctx -c npm'
alias pnpm='lean-ctx -c pnpm'
alias cargo='lean-ctx -c cargo'
alias docker='lean-ctx -c docker'
alias kubectl='lean-ctx -c kubectl'
alias gh='lean-ctx -c gh'
alias pip='lean-ctx -c pip'
alias curl='lean-ctx -c curl'
# ... and 14 more (run lean-ctx init --global for all)
```
... [lean-ctx: omitted 4 lines]
### LSP Integration (Optional)
`ctx_refactor` provides LSP-powered code intelligence (rename, references, go-to-definition, find-implementations). It requires an external language server to be installed — this is **optional** and not needed for core functionality.
... [lean-ctx: omitted 3 lines]
| Rust | `rust-analyzer` | `rustup component add rust-analyzer` |
| TypeScript/JS | `typescript-language-server` | `npm i -g typescript-language-server typescript` |
... [lean-ctx: omitted 7 lines]
```toml
[lsp]
rust = "/opt/custom/rust-analyzer"
python = "~/.venvs/main/bin/pylsp"
```
Without language servers, `ctx_search`, `ctx_symbol`, `ctx_graph`, and `ctx_callgraph` still provide powerful code navigation — LSP adds semantic precision for complex refactorings.
## Persistent Stats & Web Dashboard
lean-ctx tracks all compressions (both MCP tools and shell hook) in `~/.lean-ctx/stats.json`:
- Per-command breakdown with token counts and USD estimates ($2.50/1M tokens, aligned with MCP)
... [lean-ctx: omitted 5 lines]
```bash
lean-ctx gain             # Visual dashboard (colors, bars, sparklines)
lean-ctx gain --graph     # 30-day savings chart
lean-ctx gain --daily     # Bordered day-by-day table with USD
lean-ctx gain --json      # Raw JSON export
```
... [lean-ctx: omitted 6 lines]
- 5 interactive charts (cumulative savings, daily rate, activity, top commands, distribution)
... [lean-ctx: omitted 3 lines]
## lean-ctx vs RTK
| Feature | RTK | lean-ctx |
... [lean-ctx: omitted 3 lines]
| **CLI compression** | ~50 commands | **95+ patterns** (git, npm, cargo, docker, gh, kubectl, pip, ruff, eslint, prettier, tsc, go, terraform, make, maven, gradle, dotnet, flutter, dart, poetry, uv, playwright, rubocop, bundle, vitest, aws, psql, mysql, prisma, helm, bun, deno, swift, zig, cmake, ansible, composer, mix, bazel, systemd, curl, wget, JSON, logs...) |
| **File reading** | `rtk read` (signatures mode) | **Modes: full (cached), map, signatures, diff, aggressive, entropy, lines:N-M** |
... [lean-ctx: omitted 2 lines]
| **Dependency maps** | ✗ | ✓ import/export extraction (26 languages via tree-sitter) |
| **Context checkpoints** | ✗ | ✓ `ctx_compress` for long conversations |
... [lean-ctx: omitted 5 lines]
| **Stats & Graphs** | ✓ `rtk gain` (SQLite + ASCII graph) | ✓ Visual terminal dashboard (ANSI colors, Unicode bars, sparklines, USD) + `--graph` + `--daily` + `--json` + web dashboard |
... [lean-ctx: omitted 1 lines]
| **Editors** | Claude Code, OpenCode, Gemini CLI | **All MCP editors (Cursor, Copilot, Claude Code, Windsurf, Codex, Antigravity, OpenCode) + shell hook (OpenClaw, any terminal)** |
... [lean-ctx: omitted 1 lines]
| **History analysis** | ✗ | ✓ `lean-ctx discover` — find uncompressed commands |
... [lean-ctx: omitted 3 lines]
| **LITM-aware positioning** | ✗ | ✓ Attention-optimal context placement (primacy/recency) |
... [lean-ctx: omitted 1 lines]
| **Real project benchmarks** | ✗ | ✓ `lean-ctx benchmark run` — scans project files, measures tokens/latency/quality |
**Key difference**: RTK compresses CLI output only. lean-ctx compresses CLI output *and* file reads, search results, and project context through the MCP protocol — reaching up to 99% savings on cached re-reads and 60-90% on CLI output. With CCP (v2.0.0), lean-ctx additionally eliminates cold-start overhead by persisting session state across conversations.
## tree-sitter Signature Engine
Since v1.5.0, lean-ctx uses [tree-sitter](https://tree-sitter.github.io/tree-sitter/) for AST-based signature extraction (enabled by default). This replaces the previous regex-based extractor with accurate parsing of multi-line signatures, arrow functions, and nested definitions.
**26 languages supported**: TypeScript, JavaScript, Rust, Python, Go, Java, C, C++, Ruby, C#, Kotlin, Swift, PHP, Bash, Dart, Scala, Elixir, Zig, GDScript, Lua, Luau, OCaml, Haskell, Julia, Solidity, Nix — plus signature extraction from embedded Vue/Svelte `<script>` blocks.
... [lean-ctx: omitted 4 lines]
| Nested classes/methods | Heuristic | AST scope tracking |
... [lean-ctx: omitted 2 lines]
```bash
cargo install lean-ctx --no-default-features
```
## Uninstall
```bash
# Remove shell aliases
lean-ctx init --global  # re-run to see what was added, then remove from shell profile

# Remove binary
cargo uninstall lean-ctx

# Remove stats
rm -rf ~/.lean-ctx
```
## Contributing
Contributions welcome! Please open an issue or PR on [GitHub](https://github.com/yvgude/lean-ctx).
... [lean-ctx: omitted 1 lines]
- [Buy me a coffee](https://buymeacoffee.com/yvgude)
## Security
lean-ctx is a **privacy-first** tool — no tracking, no analytics, no PII collection. Optional network activity (daily version check, opt-in anonymous stats) is fully disableable. See [SECURITY.md](SECURITY.md) for:
... [lean-ctx: omitted 3 lines]
- VirusTotal false positive explanation (common with Rust binaries)
... [lean-ctx: omitted 1 lines]
> **Note on VirusTotal:** Rust binaries are frequently flagged by ML-based heuristic scanners (e.g., Microsoft's `Wacatac.B!ml`). This is a [known issue](https://users.rust-lang.org/t/rust-programs-flagged-as-malware/49799) affecting many Rust projects. 1/72 engines flagging = false positive. Build from source with `cargo install lean-ctx` to verify.
## License
Apache-2.0 — see [LICENSE](../LICENSE) for details.

