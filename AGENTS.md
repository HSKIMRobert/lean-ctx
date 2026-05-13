# lean-ctx — Context Engineering Layer

lean-ctx optimizes LLM context by compressing file reads, shell output, and search results.

## Integration Mode: Hybrid

- **Reads/Search** → MCP tools (`ctx_read`, `ctx_search`) for caching + compression
- **Shell commands** → CLI hooks rewrite to `lean-ctx -c "…"` for pattern compression
- **File editing** → native Edit/StrReplace (lean-ctx only handles READ operations)

## MCP tools (use for reads)

| Tool | Purpose |
|------|---------|
| `ctx_read(path, mode)` | Cached, compressed file reads (10 modes) |
| `ctx_search(pattern, path)` | Token-efficient code search |

## CLI commands (use for shell)

```bash
lean-ctx -c "git status"     # compressed shell output
lean-ctx -c "cargo test"     # compressed test output
lean-ctx ls src/              # directory map
```

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
