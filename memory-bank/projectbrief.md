# lean-ctx — Project Brief

## Ziel
`lean-ctx` ist eine **Context Runtime for AI Agents**: ein lokales, auditierbares Single-Binary, das Agenten zuverlässig mit **relevantem Kontext**, **Workflows/Rails**, **Evidence**, **Knowledge** und **Multi-Agent Coordination** versorgt – bei minimalen Token-Kosten.

## Nicht-Ziele / Prinzipien
- **Keine Tiers / Paywalls**: alles für alle.
- **Local-first, zero telemetry**: keine verpflichtende Cloud, keine heimlichen Netzwerkcalls.
- **Qualität vor Scope**: SSOT/CI-Gates verhindern Drift; Features müssen end-to-end nutzbar sein (keine „toten" Tools ohne Instrumentation).

## Kernversprechen
- **MCP Tooling**: Granular **42 Tools** + Unified **5 Tools** (SSOT via `rust/src/tool_defs.rs` → `website/generated/mcp-tools.json`).
- **Read Modes**: **10** (`auto`, `full`, `map`, `signatures`, `diff`, `aggressive`, `entropy`, `task`, `reference`, `lines:N-M`).
- **Agent Rails**: Workflow State Machine + Tool Gatekeeper + Evidence Store (`ctx_workflow`), damit Agenten prozess-konform arbeiten.
- **Observability**: `ctx_cost` + `ctx_heatmap` default-on (lokal), Retention-Limits, deterministische Token-Zählung.

## Lieferobjekte
- Rust crate `lean_ctx` (Library-first) + Binary `lean-ctx`.
- Website (Astro) liegt im separaten Deploy-Worktree/Branch; zählt/darstellt nur Manifest/SSOT-getrieben.

## Target Audience
- Developers mit AI Coding Assistants (Cursor, Claude Code, GitHub Copilot, Windsurf, etc.)
- Power User die LLM-Performance und API-Kosten optimieren
- AI-Agent-Orchestratoren/Harness-Entwickler

## Repository
- **Primary**: GitHub — https://github.com/yvgude/lean-ctx
- **Mirror**: GitLab — https://gitlab.pounce.ch/root/lean-ctx
- **Website**: https://leanctx.com
- **Crate**: https://crates.io/crates/lean-ctx
