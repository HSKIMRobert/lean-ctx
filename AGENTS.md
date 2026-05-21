# lean-ctx — Context Engineering Layer

lean-ctx optimizes LLM context by compressing file reads, shell output, and search results.

## Integration Mode: Hybrid

- **Reads/Search** → MCP tools (`ctx_read`, `ctx_search`) for caching + compression
- **Shell commands** → `lean-ctx -c "…"` via CLI (preferred) or `ctx_shell` via MCP (both work)
- **File editing** → native Edit/StrReplace (lean-ctx only handles READ operations)

## MCP tools (use for reads)

| Tool | Purpose |
|------|---------|
| `ctx_read(path, mode)` | Cached, compressed file reads (10 modes) |
| `ctx_search(pattern, path)` | Token-efficient code search |
| `ctx_shell(command)` | Compressed shell output (alternative to CLI) |

## CLI commands (optimized shell, lower overhead)

```bash
lean-ctx -c "git status"     # compressed shell output
lean-ctx -c "cargo test"     # compressed test output
lean-ctx ls src/              # directory map
```

## Development Workflow

When working on lean-ctx itself:

1. **Before building**: `lean-ctx stop` (LaunchAgent respawns otherwise)
2. **Build**: `cd rust && cargo build --release`
3. **Test**: `cargo test --lib` + `cargo clippy -- -W clippy::all`
4. **Install**: `lean-ctx dev-install` (atomic stop→build→install→restart)

## Session Continuity

Read `memory-bank/activeContext.md` at session start for current state.
Append to `memory-bank/decisions.md` when making architecture decisions.

## Provider Pipeline (Context Cortex)

External data sources (GitHub, GitLab, Jira, Postgres, MCP bridges, custom REST) are first-class citizens.
All provider data flows through the same consolidation pipeline:

1. `ContextProvider::execute()` → raw `ProviderResult`
2. `consolidation::consolidate()` → `ConsolidationArtifacts` (BM25 chunks, graph edges, knowledge facts, cache entries)
3. `apply_artifacts_to_stores()` → persists to BM25 index, Graph index, ProjectKnowledge, Session cache (background thread)

This means `ctx_semantic_search` finds issues/PRs/tickets, `ctx_knowledge` recalls provider facts,
and `ctx_read` shows cross-source hints (e.g. "Issue #42 references this file").

## Quality Bar

- Zero clippy warnings, all tests pass
- Security: PathJail, Shell Allowlist, bounded_lock, no hardcoded secrets
- No mock data, no placeholders, no stubs

<!-- lean-ctx -->
## lean-ctx

Prefer lean-ctx MCP tools over native equivalents for token savings.
Full rules: @LEAN-CTX.md
<!-- /lean-ctx -->
<!-- lean-ctx-compression -->
OUTPUT STYLE: dense
- Each statement = one atomic fact line
- Use abbreviations: fn, cfg, impl, deps, req, res, ctx, err, ret
- Diff lines only (+/-/~), never repeat unchanged code
- Symbols: → (causes), + (adds), − (removes), ~ (modifies), ∴ (therefore)
- No narration, no filler, no hedging
- BUDGET: ≤200 tokens per response unless code block required
<!-- /lean-ctx-compression -->
