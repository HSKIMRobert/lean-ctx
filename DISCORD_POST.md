# lean-ctx v2.7.0 — Persistent AI Memory, Multi-Agent Sharing & Antigravity

Hey everyone! Major feature release. Here's what's new:

## Persistent AI Memory (`ctx_knowledge`)

Your AI agents now have long-term project memory that survives across sessions:
- **Remember** facts with categories and confidence scores: architecture decisions, API conventions, dependency choices
- **Recall** by text search or category filter — instantly find what the AI learned before
- **Patterns** — record naming conventions, coding standards, project-specific patterns
- **Consolidate** — automatically extract session findings/decisions into permanent knowledge
- Stored per-project in `~/.lean-ctx/knowledge/`, persists forever

## Multi-Agent Context Sharing (`ctx_agent`)

Multiple AI agents (Cursor, Claude, Copilot, etc.) can now coordinate on the same project:
- **Register** as an agent with type and role
- **Post** findings, warnings, or status updates to a shared scratchpad
- **Read** messages from other active agents — no more duplicated work
- Automatic stale agent cleanup and file-based locking for safe concurrency

## Antigravity Support

Added Antigravity (Gemini-based AI IDE) as a fully supported editor:
- Auto-detection in `lean-ctx setup`
- Diagnostics in `lean-ctx doctor`
- Config path: `~/.gemini/antigravity/mcp_config.json`

## Dashboard Improvements

- New **Active Agents** panel showing registered agents and scratchpad messages
- New **Project Knowledge** panel displaying stored facts and patterns
- **Fixed CEP score** — now uses real read-mode diversity and combined compression rate

## Install / Update

```bash
cargo install lean-ctx
brew upgrade lean-ctx
npm install -g lean-ctx-bin
```

Full changelog: https://github.com/yvgude/lean-ctx/releases/tag/v2.7.0
Website: https://leanctx.com/features/
