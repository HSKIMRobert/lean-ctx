```
  ██╗     ███████╗ █████╗ ███╗   ██╗     ██████╗████████╗██╗  ██╗
  ██║     ██╔════╝██╔══██╗████╗  ██║    ██╔════╝╚══██╔══╝╚██╗██╔╝
  ██║     █████╗  ███████║██╔██╗ ██║    ██║        ██║    ╚███╔╝ 
  ██║     ██╔══╝  ██╔══██║██║╚██╗██║    ██║        ██║    ██╔██╗ 
  ███████╗███████╗██║  ██║██║ ╚████║    ╚██████╗   ██║   ██╔╝ ██╗
  ╚══════╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═══╝     ╚═════╝   ╚═╝   ╚═╝  ╚═╝
             Context Runtime for AI Agents
```

<h3 align="center">The context layer for AI coding agents</h3>

<p align="center">
  <strong>Reduce token waste in Cursor, Claude Code, Copilot, Windsurf, Codex, Gemini & more by 60–95% (up to 99% on cached reads)</strong><br/>
  Shell Hook + MCP Server · 51 tools · 10 read modes · 56 pattern modules + 270 passthrough rules · Tree-sitter AST for 21 languages · Single Rust binary<br/>
  <strong>Context Intelligence:</strong> Bounce detection, context gate with graph/intent/knowledge-based mode routing, MCP resources &amp; prompts, dynamic tool categories, client capability detection across 29+ AI agents
</p>

<p align="center">
  <a href="https://github.com/yvgude/lean-ctx/actions/workflows/ci.yml"><img src="https://github.com/yvgude/lean-ctx/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/yvgude/lean-ctx/actions/workflows/security-check.yml"><img src="https://github.com/yvgude/lean-ctx/actions/workflows/security-check.yml/badge.svg" alt="Security"></a>
  <a href="https://crates.io/crates/lean-ctx"><img src="https://img.shields.io/crates/v/lean-ctx?color=%23e6522c" alt="crates.io"></a>
  <a href="https://crates.io/crates/lean-ctx"><img src="https://img.shields.io/crates/d/lean-ctx?color=%23e6522c" alt="Downloads"></a>
  <a href="https://www.npmjs.com/package/lean-ctx-bin"><img src="https://img.shields.io/npm/v/lean-ctx-bin?label=npm&color=%23cb3837" alt="npm"></a>
  <a href="https://aur.archlinux.org/packages/lean-ctx"><img src="https://img.shields.io/aur/version/lean-ctx?color=%231793d1" alt="AUR"></a>
  <a href="https://pi.dev/packages/pi-lean-ctx"><img src="https://img.shields.io/badge/Pi.dev-pi--lean--ctx-6366f1?logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyNCIgaGVpZ2h0PSIyNCIgdmlld0JveD0iMCAwIDI0IDI0IiBmaWxsPSJ3aGl0ZSI+PHRleHQgeD0iNCIgeT0iMTgiIGZvbnQtc2l6ZT0iMTYiIGZvbnQtZmFtaWx5PSJzZXJpZiI+z4A8L3RleHQ+PC9zdmc+" alt="Pi.dev"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache%202.0-blue.svg" alt="License"></a>
  <a href="https://discord.gg/pTHkG9Hew9"><img src="https://img.shields.io/badge/Discord-Join-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  <a href="https://x.com/leanctx"><img src="https://img.shields.io/badge/𝕏-Follow-000000?logo=x&logoColor=white" alt="X/Twitter"></a>
  <img src="https://img.shields.io/badge/Telemetry-Opt--in%20Only-brightgreen?logo=shield&logoColor=white" alt="Opt-in Telemetry">
</p>

<p align="center">
  <a href="https://leanctx.com">Website</a> ·
  <a href="https://leanctx.com/docs/getting-started">Docs</a> ·
  <a href="#get-started-60-seconds">Install</a> ·
  <a href="#demo">Demo</a> ·
  <a href="#benchmarks">Benchmarks</a> ·
  <a href="cookbook/README.md">Cookbook</a> ·
  <a href="SECURITY.md">Security</a> ·
  <a href="CHANGELOG.md">Changelog</a> ·
  <a href="https://discord.gg/pTHkG9Hew9">Discord</a>
</p>

---

> **lean-ctx** is a local-first context runtime that compresses file reads + shell output before they reach the LLM. Cached re-reads drop to **~13 tokens**.

<p align="center"><strong>See it in action:</strong></p>

<table>
  <tr>
    <td align="center" width="33%">
      <img src="assets/leanctx-demo.gif" width="320" alt="Map-mode file read + compressed git output demo">
      <br/>
      <strong>Read + Shell</strong>
      <br/>
      Map-mode reads + compressed CLI output
    </td>
    <td align="center" width="33%">
      <img src="assets/leanctx-gain.gif" width="320" alt="lean-ctx gain live dashboard demo">
      <br/>
      <strong>Gain (live)</strong>
      <br/>
      Tokens + USD savings in real time
    </td>
    <td align="center" width="33%">
      <img src="assets/leanctx-benchmark.gif" width="320" alt="lean-ctx benchmark report demo">
      <br/>
      <strong>Benchmark proof</strong>
      <br/>
      Measure compression by language + mode
    </td>
  </tr>
</table>

<p align="center"><sub>All GIFs are generated from reproducible VHS tapes in <code>demo/</code>.</sub></p>

## What it does

**One binary replaces your entire context stack:**

| Replaces | With lean-ctx | How |
|----------|--------------|-----|
| Output compression tools | 4 compression levels + 56 pattern modules | Shell hook + terse pipeline + 270 passthrough rules |
| Context window managers | 10 read modes + auto-archive | Adaptive mode selection per file, Tree-sitter AST for 21 languages |
| Session memory tools | CCP + temporal knowledge graph | Facts with validity, cross-session recovery, episodic + procedural memory |
| Code graph tools | Property Graph + hybrid search | BM25 + embeddings + graph proximity |
| Context observability tools | Context Manager (dashboard) | Real-time token tracking, file ledger, compression stats |
| Governance / quality tools | Profiles, roles, budgets, SLOs | Context proof, verification engine, quality gates |

**Core capabilities:**

- **File reads (MCP)**: cached + mode-aware reads (`full`, `map`, `signatures`, `diff`, …) with graph-aware related files hints
- **Shell output (hook)**: compresses noisy CLI output via 56 pattern modules + 270 passthrough rules (git, npm, cargo, docker, kubectl, terraform, …)
- **Context Manager** (beta): browser-based dashboard (`lean-ctx dashboard`) with real-time context window visualization — file ledger with token counts, compression ratios, system prompt cost breakdown, conversation history weight, context utilization gauge, and compression stats
- **Graph-Powered Intelligence**: multi-edge Property Graph (imports, calls, exports, type_ref) with weighted impact analysis, hybrid search (BM25 + embeddings + graph proximity via RRF), and incremental git-diff updates
- **Governance**: profiles, roles, budgets, and SLOs — define how much context each agent uses, what tools they can access, and when to throttle
- **Context Proof & Verification** (`ctx_proof`, `ctx_verify`): cryptographic context proofs with 4-layer verification engine and quality gates (levels 0–4)
- **LSP Refactoring** (`ctx_refactor`): language-server-powered rename, references, go-to-definition, and find-implementations via rust-analyzer, typescript-language-server, pylsp, gopls — with timeout-protected channel-based IO
- **Knowledge System**: temporal knowledge graph with facts, validity windows, cross-session recovery, episodic memory (task-level summaries), and procedural memory (learned workflows)
- **Multi-Agent** (`ctx_agent`, `ctx_handoff`): agent handoff with context transfer bundles, diary system (discovery/decision/blocker/progress/insight), and synchronized shared state
- **Archive Full-Text Search** (`ctx_expand search_all`): FTS5-powered cross-archive search over all previously archived tool outputs
- **PR Context Packs**: `lean-ctx pack --pr` builds a PR-ready context pack (changed files, related tests, impact, artifacts)
- **Context Packages**: `lean-ctx pack create` bundles Knowledge + Graph + Session + Gotchas into portable `.lctxpkg` files — share context across projects/teams with SHA-256 integrity, auto-load on session start, and smart merge (dedup facts, overlay graph)
- **Session memory (CCP)**: persist task/facts/decisions across chats with structured recovery queries surviving compaction
- **Observability**: `lean-ctx gain --live` for real-time savings, `lean-ctx wrapped` for weekly/monthly summaries, `lean-ctx watch` for TUI monitoring, heatmaps, and slow-log analysis
- **HTTP mode**: `lean-ctx serve` for Streamable HTTP MCP + `/v1/tools/call` (used by the Cookbook + SDK)

## How it works (30 seconds)

```
AI tool  →  (MCP tools + shell commands)  →  lean-ctx  →  your repo + CLI
```

- **MCP server**: exposes `ctx_*` tools (read modes, caching, deltas, search, memory, multi-agent)
- **Shell hook**: transparently compresses common commands so the LLM sees less noise
- **Property Graph**: multi-edge code graph powers impact analysis, related file discovery, and search ranking
- **CCP**: persists session state with structured recovery queries so long-running work doesn’t “cold start” every chat
- **Context Manager**: browser dashboard for real-time visibility into what’s in your context window
- **Governance**: profiles, budgets, SLOs, and verification proofs for enterprise-grade context control

## Get started (60 seconds)

```bash
# 1) Install (pick one)
curl -fsSL https://leanctx.com/install.sh | sh      # universal (no Rust needed)
brew tap yvgude/lean-ctx && brew install lean-ctx    # macOS / Linux
npm install -g lean-ctx-bin                          # Node.js
cargo install lean-ctx                               # Rust
pi install npm:pi-lean-ctx                           # Pi Coding Agent

# 2) Setup (shell + auto-detected AI tools)
lean-ctx setup

# 3) Verify
lean-ctx doctor

# 4) See the payoff
lean-ctx gain --live
lean-ctx wrapped --week
```

After `setup`, restart your shell and your editor/AI tool once so the MCP + hooks are active.

<details>
<summary><strong>Troubleshooting / Safety</strong></summary>

- Disable immediately (current shell): `lean-ctx-off`
- Run a single command uncompressed: `lean-ctx -c --raw "git status"`
- Only activate in AI agent sessions: set `shell_activation = "agents-only"` in `~/.config/lean-ctx/config.toml`
- Per-project config override: create `.lean-ctx.toml` in your project root (auto-merged with global config)
- Docker projects sharing `/workspace`: create `.lean-ctx-id` with a unique name to prevent context collisions
- Update: `lean-ctx update`
- Diagnose (shareable): `lean-ctx doctor --json`

</details>

## Supported IDEs & AI tools

lean-ctx is a standard **MCP server**, so it works with any MCP-compatible client. Two integration modes are auto-selected per agent:

| Mode | How it works | Best for |
|---|---|---|
| **Hybrid** | MCP for cached reads (~13 tokens) + shell hooks for command compression | Agents with shell access (Cursor, Claude Code, Codex, …) |
| **MCP** | All 51 tools via MCP protocol, no shell hooks | Protocol-only agents (JetBrains, VS Code, Zed, …) |

### Agent compatibility matrix

| Agent | Hybrid | MCP | Setup |
|---|:---:|:---:|---|
| Cursor | ● | | `lean-ctx init --agent cursor` |
| Claude Code | ● | | `lean-ctx init --agent claude` |
| Codex CLI | ● | | `lean-ctx init --agent codex` |
| Gemini CLI | ● | | `lean-ctx init --agent gemini` |
| Windsurf | ● | | `lean-ctx init --agent windsurf` |
| GitHub Copilot | ● | | `lean-ctx init --agent copilot` |
| CRUSH | ● | | `lean-ctx init --agent crush` |
| Hermes | ● | | `lean-ctx init --agent hermes` |
| OpenCode | ● | | `lean-ctx init --agent opencode` |
| Pi | ● | | `lean-ctx init --agent pi` |
| Qoder | ● | | `lean-ctx init --agent qoder` |
| Amp | ● | | `lean-ctx init --agent amp` |
| Cline | ● | | `lean-ctx init --agent cline` |
| Roo Code | ● | | `lean-ctx init --agent roo` |
| Kiro | ● | | `lean-ctx init --agent kiro` |
| Antigravity | ● | | `lean-ctx init --agent antigravity` |
| Amazon Q | ● | | `lean-ctx init --agent amazonq` |
| Qwen | ● | | `lean-ctx init --agent qwen` |
| Trae | ● | | `lean-ctx init --agent trae` |
| Verdent | ● | | `lean-ctx init --agent verdent` |
| Aider | | ● | `lean-ctx init --agent aider` |
| Continue | | ● | `lean-ctx init --agent continue` |
| JetBrains IDEs | | ● | `lean-ctx init --agent jetbrains` |
| QoderWork | | ● | `lean-ctx init --agent qoderwork` |
| VS Code | | ● | `lean-ctx init --agent vscode` |
| Zed | | ● | `lean-ctx init --agent zed` |
| Neovim | | ● | `lean-ctx init --agent neovim` |
| Emacs | | ● | `lean-ctx init --agent emacs` |
| Sublime Text | | ● | `lean-ctx init --agent sublime` |

> **Any MCP-compatible client** works out of the box — the table above shows agents with first-class auto-setup.

### When to use (and when not to)

**Great fit if you…**
- use AI coding tools daily and your sessions are shell-heavy (git/tests/builds)
- work in medium/large repos (50+ files / monorepos)
- want a local-first layer with **no telemetry by default**

**Skip it if you…**
- mostly work in tiny repos and rarely call the shell from your AI tool
- always need raw/unfiltered logs (you can still use `--raw`, but ROI is lower)

<a id="demo"></a>

## Demo

Try these in any repo:

```bash
lean-ctx read rust/src/server/mod.rs -m map
lean-ctx -c "git log -n 5 --oneline"
lean-ctx gain --live
lean-ctx dashboard                              # Context Manager (browser)
lean-ctx watch                                  # TUI monitor
lean-ctx benchmark report .
```

- The repo ships the exact tapes used to render the GIFs in `demo/`
- Regenerate locally:

```bash
vhs demo/leanctx.tape
vhs demo/gain.tape
vhs demo/benchmark.tape
```

<a id="benchmarks"></a>

## Benchmarks

- **Latest snapshot**: [BENCHMARKS.md](BENCHMARKS.md)
- **Reproduce**:

```bash
lean-ctx benchmark report .
```

## Docs

- Getting started: https://leanctx.com/docs/getting-started
- Tools reference: https://leanctx.com/docs/tools/
- CLI reference: https://leanctx.com/docs/cli-reference/
- Comparison (vs RTK, Context+, MemGPT): https://leanctx.com/compare/
- FAQ: [discord-faq.md](discord-faq.md)
- Feature catalog (SSOT snapshot): [LEANCTX_FEATURE_CATALOG.md](LEANCTX_FEATURE_CATALOG.md)
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md)
- Vision: [VISION.md](VISION.md)

## Privacy & security

- **No telemetry by default**
- **Optional anonymous stats sharing** (opt-in during setup)
- **Disableable update check** (config `update_check_disabled = true` or `LEAN_CTX_NO_UPDATE_CHECK=1`)
- **40+ security hardening fixes** in v3.5.16 (path traversal, injection, CSPRNG, CSP, resource limits — [details](CHANGELOG.md))
- Runs locally; your code never leaves your machine unless you explicitly enable cloud sync

See [SECURITY.md](SECURITY.md).

## Uninstall

```bash
lean-ctx-off       # disable immediately (current shell session)
lean-ctx uninstall # remove hooks + editor configs + data dir

# Remove the binary (pick your install method)
brew uninstall lean-ctx
npm uninstall -g lean-ctx-bin
cargo uninstall lean-ctx
pi uninstall npm:pi-lean-ctx                        # Pi Coding Agent
```

## Contributing

Start with [CONTRIBUTING.md](CONTRIBUTING.md). Easy first PR: propose a new CLI compression pattern via the [issue template](.github/ISSUE_TEMPLATE/compression_pattern.md).

## License

Apache License 2.0 — see [LICENSE](LICENSE).

Portions of this software were originally released under the MIT License. See [LICENSE-MIT](LICENSE-MIT) and [NOTICE](NOTICE).