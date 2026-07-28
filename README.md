README.md [696L]
<div align="center">
<pre>
██╗     ███████╗ █████╗ ███╗   ██╗     ██████╗████████╗██╗  ██╗
... [lean-ctx: omitted 5 lines]
</pre>
### **Control what your AI can see.**
**LeanCTX — Lean Context Engineering for AI agents**
... [lean-ctx: omitted 11 lines]
|---------|-------------|
| Repeated file reads: ~2000 tokens each | Cached re-reads: **~13 tokens** |
| Raw `git status`: ~800 tokens | Compressed: **~120 tokens** |
| Every turn re-sends the whole history | Proxy compresses each request, **prompt-cache-safe** |
... [lean-ctx: omitted 1 lines]
| No visibility into context usage | Real-time dashboard + budget control |
... [lean-ctx: omitted 2 lines]
  <a href="https://github.com/yvgude/lean-ctx/stargazers"><img src="https://img.shields.io/github/stars/yvgude/lean-ctx?style=social" alt="GitHub Stars"></a>&nbsp;&nbsp;
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
... [lean-ctx: omitted 2 lines]
  <a href="https://leanctx.com">Website</a>&nbsp;&nbsp;·&nbsp;&nbsp;<a href="https://leanctx.com/docs/getting-started">Docs</a>&nbsp;&nbsp;·&nbsp;&nbsp;<a href="#get-started-60-seconds">Install</a>&nbsp;&nbsp;·&nbsp;&nbsp;<a href="#use-it-from-your-own-code-sdks">SDKs</a>&nbsp;&nbsp;·&nbsp;&nbsp;<a href="#real-world-scenarios">Scenarios</a>&nbsp;&nbsp;·&nbsp;&nbsp;<a href="#demo">Demo</a>&nbsp;&nbsp;·&nbsp;&nbsp;<a href="#benchmarks">Benchmarks</a>&nbsp;&nbsp;·&nbsp;&nbsp;<a href="cookbook/README.md">Cookbook</a>&nbsp;&nbsp;·&nbsp;&nbsp;<a href="SECURITY.md">Security</a>&nbsp;&nbsp;·&nbsp;&nbsp;<a href="CHANGELOG.md">Changelog</a>
... [lean-ctx: omitted 3 lines]
> **Control what your AI can see.** LeanCTX — short for **Lean Context** — is the **context engineering layer** for AI agents: one local Rust binary that decides what your agents read, compresses what they send to the model, remembers what they learn, guards what they touch — and proves what they save.
> Token savings are the receipt. Intelligence is the product. Works with **Cursor, Claude Code, Copilot, Windsurf, Codex, Gemini** and 30+ other agents — no config needed.
... [lean-ctx: omitted 4 lines]
      <img src="assets/leanctx-demo.gif" width="320" alt="Map-mode file read + compressed git output demo">
... [lean-ctx: omitted 6 lines]
      <img src="assets/leanctx-gain.gif" width="320" alt="lean-ctx gain live dashboard demo">
... [lean-ctx: omitted 6 lines]
      <img src="assets/leanctx-benchmark.gif" width="320" alt="lean-ctx benchmark report demo">
... [lean-ctx: omitted 7 lines]
<p align="center"><sub>All GIFs are generated from reproducible VHS tapes in <code>demo/</code>.</sub></p>
## Why developers use LeanCTX
- **Longer useful coding sessions** — less context waste = more room for actual code reasoning
... [lean-ctx: omitted 2 lines]
- **Works with your existing setup** — one `lean-ctx setup` command, no config changes needed
... [lean-ctx: omitted 1 lines]
- **Model-agnostic & yours** — swap OpenAI/Anthropic/Gemini freely; your context and memory stay local and portable, never locked in a vendor's black box
... [lean-ctx: omitted 2 lines]
  <strong>Saves you tokens?</strong> <a href="https://github.com/yvgude/lean-ctx">Give it a star</a> — it helps others discover LeanCTX.
... [lean-ctx: omitted 2 lines]
## Why now — own your context
Models are converging on commodity. The durable edge isn't *which* model you call — it's your **context**: what your agents read, what they remember, and what you can prove. And the layer that optimizes and *owns* that context can't come from the vendor that bills per token or keeps your memory in a black box — it has to sit on your side.
That's the shift behind "agent entities" that live in your chat and remember your company (Claude in Slack, ClickUp Brain): a **context login, not a model login** — you end up renting your own company knowledge back. LeanCTX is the opposite layer. It keeps the moat yours: local-first, portable (`.ctxpkg`), and model-agnostic — swap OpenAI, Anthropic or Gemini without losing context or cache. **Own your context; don't rent it back.**
... [lean-ctx: omitted 1 lines]
## What it does — the four dimensions of context
LeanCTX treats context as a managed resource, not an afterthought. One binary
covers the four dimensions that decide how well an AI agent actually performs:
### 1. Compression — input efficiency
Your AI agent reads files and runs commands. LeanCTX compresses both automatically.
... [lean-ctx: omitted 1 lines]
- **Target density** (`density:0.4`): SDE-style budget compression — keeps the highest-entropy lines until ~40% of the original tokens remain, deterministic
- **JIT disclosure**: `signatures` carries line spans and points at `lines:N-M` for targeted expansion — outline first, bodies on demand
... [lean-ctx: omitted 2 lines]
- **Reversible by design (CCR)**: compression never *discards* content — pruned or truncated payloads move to a content-addressed store with a deterministic handle, so the model can pull the original bytes back on demand via `ctx_expand`, `ctx_retrieve`, an in-band marker, or `GET /v1/references/{id}`. [Five recovery paths →](docs/comparisons/vs-headroom.md#reversibility)
### 2. Routing — the right fidelity per read
Not every file needs the same depth. LeanCTX sends the signal, not the noise.
... [lean-ctx: omitted 1 lines]
- **Adaptive `ModePredictor`**: learns the optimal read mode per file type from past sessions
- **`IntentEngine`**: classifies query complexity so simple lookups stay cheap
### 3. Memory — context that persists
Context doesn't disappear between chats anymore.
- **Session memory (CCP)**: persist task/facts/decisions across chats — structured recovery queries survive compaction
- **Knowledge graph**: temporal facts with validity windows, episodic + procedural memory
... [lean-ctx: omitted 2 lines]
### 4. Verification — control what reaches the model
Performance is accuracy, not just speed. You stay in control of the window.
... [lean-ctx: omitted 5 lines]
- **Web & Research** (`ctx_url_read`): pull a public web page, PDF, or YouTube transcript into context as compressed, citation-backed text — `facts`/`quotes` return claims with a confidence score + source URL, relevance-ranked research-compression distils to a token budget, SSRF-guarded (http/https only)
... [lean-ctx: omitted 1 lines]
- **LSP Refactoring** (`ctx_refactor`): language-server-powered rename, references, go-to-definition via rust-analyzer, typescript-language-server, pylsp, gopls
... [lean-ctx: omitted 1 lines]
- **Archive Full-Text Search** (`ctx_expand search_all`): FTS5-powered cross-archive search over all previously archived tool outputs
... [lean-ctx: omitted 1 lines]
- **Context Packages**: `lean-ctx pack create` bundles Knowledge + Graph + Session into portable `.ctxpkg` files with SHA-256 integrity
- **Context Time Machine**: `lean-ctx snapshot create|list|show|verify|restore|publish|import` — git-anchored, ed25519-signed snapshots of the layer state (lineage, ledger Φ, ROI, session) on an append-only timeline; replay them in the dashboard, `restore` to resume a session (and `--git` to check out the commit), or `publish`/`import` a signed snapshot to share it ([concept →](docs/concepts/context-time-machine.md))
- **Observability**: `lean-ctx gain --live` for real-time savings, `lean-ctx wrapped` for weekly/monthly summaries (`gain --svg`/`--share` for a shareable card or self-hostable page), `lean-ctx watch` for TUI monitoring
- **Verified savings**: `lean-ctx savings` is an auditable, per-event ledger (tokenizer transparency, bounce-netting, tamper-evident SHA-256 chain) — local-only, on by default
... [lean-ctx: omitted 2 lines]
## Addons — run the ecosystem through one gateway
You don't have to choose between LeanCTX and the other context tools you already
... [lean-ctx: omitted 3 lines]
```bash
lean-ctx addon search memory   # browse the registry by category
lean-ctx addon add headroom    # installs the upstream package + wires the MCP server, on add
lean-ctx addon list            # what's wired into your gateway
```
... [lean-ctx: omitted 1 lines]
- **Folded in, not just proxied** — opt-in post-processing runs addon output through the same pipeline as your code: compress to a budget, spill oversized blobs to a `ctx_expand` handle, index into BM25 / graph / knowledge. Typed adapters route specific tools straight into `ctx_expand`, `ctx_callgraph` and `ctx_knowledge`.
... [lean-ctx: omitted 5 lines]
## Where it's going
LeanCTX is growing from a single context *layer* into a full **cognitive context
... [lean-ctx: omitted 2 lines]
- **Context Time Machine → hosted history** — the snapshot engine, dashboard replay, restore, and signed file-based share/import have shipped (see above); next is a `ctxpkg.com` registry for hosted, versioned context history and a side-by-side model-view ｜ git-diff replay. The temporal axis through everything LeanCTX does — it *decides, remembers, guards, proves, and replays*. ([concept →](docs/concepts/context-time-machine.md))
- **Context as Code** — declarative pipelines, profiles, and policies in TOML, versioned like infrastructure
... [lean-ctx: omitted 1 lines]
- **Agent Harness** — roles, budgets, and tool permissions for multi-agent governance
- **Context Observability** — SLOs on context consumption, anomaly detection, OpenTelemetry / Prometheus export
... [lean-ctx: omitted 1 lines]
## How it works (30 seconds)
LeanCTX works on **two planes** — what your agents *read* and what they *send to the model*:
... [lean-ctx: omitted 4 lines]
- **MCP server** *(read path)*: exposes `ctx_*` tools (read modes, caching, deltas, search, memory, multi-agent)
... [lean-ctx: omitted 1 lines]
- **Request proxy** *(wire path, opt-in)*: `lean-ctx proxy enable` puts a local proxy between your agent and the model that compresses **every request** — system prompt, full history and tool results — prompt-cache-safe, with measured USD spend. It can also pin **one reasoning-effort level across OpenAI, Anthropic & Gemini** (`proxy.effort`) without breaking that cache, cut **output** tokens with a cache-safe verbosity steer plus a measured holdout, and **relocate volatile fields** (dates, UUIDs, commit SHAs) out of the cacheable prefix so a stable system prompt finally caches. Every rewrite is reversible (content-addressed recovery) and byte-stable by contract. Same layer as a standalone request-compression proxy (e.g. Headroom) — you don't need one on top.
- **Property Graph**: multi-edge code graph powers impact analysis, related file discovery, and search ranking
- **Session memory**: persists state with structured recovery so long-running work never "cold starts"
... [lean-ctx: omitted 1 lines]
## Get started (30 seconds)
```bash
# 1) Install (pick one)
curl -fsSL https://leanctx.com/install.sh | sh      # universal (no Rust needed)
brew tap yvgude/lean-ctx && brew install lean-ctx    # macOS / Linux
npm install -g lean-ctx-bin                          # Node.js
cargo install lean-ctx                               # Rust
pi install npm:pi-lean-ctx                           # Pi Coding Agent

# 2) One-command setup for your agent
lean-ctx wrap cursor      # or: wrap claude / wrap codex / wrap vscode

# Done. Savings appear after your AI's first lean-ctx call.
lean-ctx gain
```
... [lean-ctx: omitted 19 lines]
## Use it from your own code (SDKs)
Beyond the CLI, lean-ctx ships published libraries so you can call it directly from your app.
**Drop-in prompt compression — [`lean-ctx-sdk`](https://pypi.org/project/lean-ctx-sdk/) ([npm](https://www.npmjs.com/package/lean-ctx-sdk)).** Compress a chat-style `messages` array before it reaches any model — deterministic and prompt-cache friendly; images, tool-calls and ids pass through untouched.
```python
# pip install lean-ctx-sdk
from lean_ctx import compress
messages = compress(messages, model="claude-sonnet-4")
```
... [lean-ctx: omitted 5 lines]
Framework adapters included (LiteLLM, LangChain, Vercel AI SDK). → **[compress() cookbook](docs/guides/compress-sdk.md)**
**Thin `/v1` contract clients — [`lean-ctx-client`](https://pypi.org/project/lean-ctx-client/) ([npm](https://www.npmjs.com/package/lean-ctx-client) · [crates.io](https://crates.io/crates/lean-ctx-client)).** Wrap the full `/v1` tool, event and session API over the process boundary — never links the engine, so it stays stable as lean-ctx evolves.
... [lean-ctx: omitted 6 lines]
## Real-world scenarios
LeanCTX grows with you. Below are the journeys most people actually take — each
links to a complete, function-by-function walkthrough in the
**[Reference](docs/reference/README.md)** (every CLI command and all 79 MCP
... [lean-ctx: omitted 4 lines]
### 🟢 Your first 30 seconds
*"I just installed it — now what?"*
```bash
lean-ctx wrap cursor  # one-command setup for your agent
lean-ctx doctor       # confirm you're wired up
```
One command installs hooks, MCP registration, and verifies the connection.
... [lean-ctx: omitted 3 lines]
### 📖 Coding every day
*"Stop re-reading the same files."*
```bash
lean-ctx read src/server.rs -m map   # API surface, ~13 tok on re-read
lean-ctx -c "git status"             # compressed shell output
```
... [lean-ctx: omitted 6 lines]
### 🧠 Resume where you left off
*"My new chat forgot everything."*
```bash
lean-ctx overview                    # task-aware project recap
lean-ctx knowledge recall "auth"     # facts that survive resets
lean-ctx knowledge consolidate       # import session + compact lifecycle
lean-ctx knowledge consolidate --all # compact every project store
```
... [lean-ctx: omitted 4 lines]
### 🗺️ Understand a new codebase
*"Where does this function ripple to?"*
```bash
lean-ctx graph impact src/auth.rs    # blast radius
lean-ctx smells scan                 # code-smell hotspots
```
... [lean-ctx: omitted 6 lines]
### 🔌 Providers & multi-repo
*"Pull in GitHub issues and our Postgres schema."*
```bash
lean-ctx provider list
lean-ctx serve --root ./api --root ./web   # multi-repo
```
... [lean-ctx: omitted 4 lines]
### 🛠️ Keep it healthy
*"Update, fix, or cleanly remove."*
```bash
lean-ctx doctor --fix
lean-ctx update
```
Self-healing diagnostics; surgical uninstall that only removes its own blocks.
... [lean-ctx: omitted 5 lines]
### 🎛️ Take control of the window
*"Budget my context like a pro."*
```bash
lean-ctx plan "refactor billing" --budget 8000
lean-ctx compile --mode balanced
```
... [lean-ctx: omitted 4 lines]
### 🤝 Run a team of agents
*"Planner + coder + reviewer on one repo."*
```text
ctx_agent action=register role=dev
ctx_handoff action=create        # baton-pass with full context
```
... [lean-ctx: omitted 6 lines]
### 🏢 Share across a team / CI
*"One shared index, headless in pipelines."*
```bash
lean-ctx team serve --config team.toml
lean-ctx bootstrap            # zero-prompt CI setup
```
... [lean-ctx: omitted 4 lines]
### 🎚️ Tune & govern
*"Make it behave exactly how we want."*
```bash
lean-ctx compression standard
lean-ctx harden               # enforce token discipline
```
... [lean-ctx: omitted 6 lines]
### 📊 Prove the payoff
*"Show me the numbers."*
```bash
lean-ctx gain --deep          # savings, cost, per-agent, heatmap
lean-ctx wrapped              # shareable recap (also: gain --svg / gain --share)
lean-ctx savings              # verified per-event ledger (auditable; savings verify)
```
... [lean-ctx: omitted 4 lines]
### 📚 The full reference
*"I want to read everything."*
Every command and all 82 MCP tools, organized as user journeys, plus
appendices for the [CLI map](docs/reference/appendix-cli-map.md),
[MCP tools](docs/reference/appendix-mcp-tools.md), and
... [lean-ctx: omitted 1 lines]
→ **[Reference index](docs/reference/README.md)**
... [lean-ctx: omitted 3 lines]
## Supported IDEs & AI tools
LeanCTX is a standard **MCP server**, so it works with any MCP-compatible client. Two integration modes are auto-selected per agent:
... [lean-ctx: omitted 3 lines]
| **MCP** | All 80 tools via MCP protocol, no shell hooks | Protocol-only agents (JetBrains, VS Code, Zed, ...) |
### Agent compatibility matrix
| Agent | Hybrid | MCP | Setup |
... [lean-ctx: omitted 1 lines]
| Cursor | ● | | `lean-ctx init --agent cursor` |
| Claude Code | ● | | `lean-ctx init --agent claude` |
| CodeBuddy | ● | | `lean-ctx init --agent codebuddy` |
| Augment CLI / VS Code | ● | | `lean-ctx init --agent augment` |
| Codex CLI | ● | | `lean-ctx init --agent codex` |
| Grok | ● | | `lean-ctx init --agent grok` |
| Gemini CLI | ● | | `lean-ctx init --agent gemini` |
| Windsurf | ● | | `lean-ctx init --agent windsurf` |
... [lean-ctx: omitted 3 lines]
| OpenCode | ● | | `lean-ctx init --agent opencode` |
... [lean-ctx: omitted 6 lines]
| Antigravity | ● | | `lean-ctx init --agent antigravity` |
... [lean-ctx: omitted 6 lines]
| Continue | | ● | `lean-ctx init --agent continue` |
| JetBrains IDEs | | ● | `lean-ctx init --agent jetbrains` |
| QoderWork | | ● | `lean-ctx init --agent qoderwork` |
... [lean-ctx: omitted 6 lines]
### When to use (and when not to)
**Great fit if you...**
- use AI coding tools daily and your sessions are shell-heavy (git/tests/builds)
- work in medium/large repos (50+ files / monorepos)
... [lean-ctx: omitted 3 lines]
- always need raw/unfiltered logs (you can still use `--raw`, but ROI is lower)
... [lean-ctx: omitted 1 lines]
window via the proxy/engine, not just the `ctx_*` tool layer), **context
... [lean-ctx: omitted 1 lines]
**provider pricing** (prompt-cache-priced vs. re-billed every turn). They stack
... [lean-ctx: omitted 1 lines]
See the [win vs. break-even matrix](docs/reference/14-performance-tuning.md#win-vs-break-even-at-a-glance)
... [lean-ctx: omitted 2 lines]
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
... [lean-ctx: omitted 8 lines]
## Benchmarks
Real, reproduced numbers — never estimated. Measured on this repo with the GPT-4o
tokenizer (`o200k_base`); a tool that isn't installed is reported as such, never
... [lean-ctx: omitted 5 lines]
| `signatures` | **96.7%** | 14.0K | 96% |
... [lean-ctx: omitted 1 lines]
lean-ctx's **own cost is measured too**: the CI-measured fixed per-session
... [lean-ctx: omitted 1 lines]
~3.0K tokens and gated via `lean-ctx doctor overhead --gate`. And the
... [lean-ctx: omitted 1 lines]
`lean-ctx benchmark dual-arm --json` replays a 72-turn session and prices it per
model (digest `f5ed145e61ce3689`, 99.4% input-side saving on cache-priced rails;
... [lean-ctx: omitted 3 lines]
rewrites are byte-stable by contract, so Anthropic (90%) / OpenAI (50%) prompt-cache
discounts survive compression. A deterministic **off-vs-on testbench**
... [lean-ctx: omitted 1 lines]
through a raw-dump baseline and through lean-ctx at an identical token budget, grades
... [lean-ctx: omitted 1 lines]
`FINDINGS.md` (tokens / turns / walltime / quality) plus a regressions file — with a
... [lean-ctx: omitted 3 lines]
## By the numbers
- **3,000+ GitHub stars** — and counting
- **280+ forks** — active community contributions
... [lean-ctx: omitted 1 lines]
- **30+ supported AI coding agents** — broadest MCP compatibility
... [lean-ctx: omitted 2 lines]
- **Live adoption metrics**: [leanctx.com/metrics](https://leanctx.com/metrics/) — installs, stars and savings, updated continuously
## Docs
- **Reference (every function, by user journey)**: [docs/reference/](docs/reference/README.md) — 11 journeys + CLI/MCP/config appendices
- **For AI agents / LLMs**: [llms.txt](llms.txt) — a curated, machine-readable map of lean-ctx (per the [llms.txt](https://llmstxt.org) convention)
... [lean-ctx: omitted 4 lines]
- Comparison (vs RTK, Context+, MemGPT): https://leanctx.com/compare/
... [lean-ctx: omitted 3 lines]
- Monorepo guide: [docs/guides/monorepo.md](docs/guides/monorepo.md)
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md)
... [lean-ctx: omitted 1 lines]
## Privacy & security
- **No telemetry by default**
... [lean-ctx: omitted 1 lines]
- **Disableable update check** (config `update_check_disabled = true` or `LEAN_CTX_NO_UPDATE_CHECK=1`)
- **40+ security hardening fixes** in v3.5.16 (path traversal, injection, CSPRNG, CSP, resource limits — [details](CHANGELOG.md))
- **Context Governance Benchmark self-assessment**: graded **C2 — Managed** against the 32-control [CGB v1.0-draft](https://github.com/yvgude/context-governance-benchmark) spec, gaps declared — [docs/compliance/cgb-self-assessment.md](docs/compliance/cgb-self-assessment.md)
... [lean-ctx: omitted 2 lines]
## Uninstall
One command removes **everything** — it stops all processes, then deletes hooks,
... [lean-ctx: omitted 2 lines]
```bash
lean-ctx uninstall                 # full clean removal
lean-ctx uninstall --dry-run       # preview every change, write nothing
lean-ctx uninstall --keep-config   # keep MCP configs + rules (for reinstall)
lean-ctx-off                       # or just disable for the current shell session
```
... [lean-ctx: omitted 6 lines]
```bash
brew uninstall lean-ctx        # Homebrew
cargo uninstall lean-ctx       # cargo install
npm uninstall -g lean-ctx-bin  # npm
pi uninstall npm:pi-lean-ctx   # Pi Coding Agent
```
## Star History
<a href="https://star-history.com/#yvgude/lean-ctx&Date">
... [lean-ctx: omitted 1 lines]
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=yvgude/lean-ctx&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=yvgude/lean-ctx&type=Date" />
... [lean-ctx: omitted 3 lines]
## Contributing
Start with [CONTRIBUTING.md](CONTRIBUTING.md). Easy first PR: propose a new CLI compression pattern via the [issue template](.github/ISSUE_TEMPLATE/compression_pattern.md).
## License
Apache License 2.0 — see [LICENSE](LICENSE).
--- lean-ctx: ctx_compose bundles search+read+symbols in one call ---

