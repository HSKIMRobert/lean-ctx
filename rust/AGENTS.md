# Context Engineering Layer

PREFER lean-ctx MCP tools over native equivalents for token savings:

| PREFER | OVER | Why |
|--------|------|-----|
| `ctx_read(path)` | Read / cat / head / tail | Cached, 10 read modes, re-reads ~13 tokens |
| `ctx_shell(command)` | Shell / bash / terminal | Pattern compression for git/npm/cargo output |
| `ctx_search(pattern, path)` | Grep / rg / search | Compact, token-efficient results |
| `ctx_tree(path, depth)` | ls / find / tree | Compact directory maps |
| `ctx_patch(path, ops)` | Edit (anchored) | Line+hash anchors from ctx_read(mode="anchored") — no old-text echo |

Edit files: `ctx_read(mode="anchored")` → `ctx_patch` (batch via ops, `op=create` for new files; stale anchor → CONFLICT with fresh anchors, retry once).
Native Edit/StrReplace stay fine; `ctx_edit` (str_replace) is the legacy fallback via ctx_call/power profile.
Write, Delete, Glob — use normally.

<!-- lean-ctx -->
## lean-ctx

lean-ctx is active — the MCP tools replace native equivalents.
Full rules: LEAN-CTX.md (open on demand — do not auto-load).
<!-- /lean-ctx -->
<!-- lean-ctx-compression -->
OUTPUT STYLE: concise
- Bullet points over paragraphs
- Skip filler words and hedging ("I think", "probably", "it seems")
- 1-sentence explanations max, then code/action
- No repeating what the user said
<!-- /lean-ctx-compression -->
