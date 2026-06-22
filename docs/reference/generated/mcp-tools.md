# Appendix — MCP Tools (generated)

<!-- GENERATED FILE — do not edit by hand. Run: `cargo run --example gen_docs --features dev-tools` -->

Source of truth: `rust/src/server/registry.rs` and the tool definitions it registers.

lean-ctx registers **77 MCP tools** (granular profile). Each entry below lists the tool name, what it does, and its parameters (`*` marks required).

## `ctx_agent`

Multi-agent coordination: shared message bus, persistent diaries, stigmergic scent field. Actions: register (agent_type+role), post (message+category), read (poll), status (active|idle|finished), handoff (transfer task+summary), sync (agents + messages + scent: claims/stuck/hot), claim/release (atomic file/task claim, message=target), brief (sub-agent briefing pack: message=task, priority=budget), return (distill sub-agent report into knowledge: message='category/key: value' lines), diary (log discovery/decision/blocker/progress/insight), recall_diary, diaries, list, info.

Parameters: `action`*, `agent_type`, `category`, `message`, `role`, `status`, `to_agent`

## `ctx_analyze`

Entropy analysis — recommends optimal compression mode for a file.

Parameters: `path`*

## `ctx_architecture`

Architecture analysis: action=overview→high-level; clusters|communities→groupings
layers|cycles→dependency violations; entrypoints|hotspots→risk areas; health→quality.
Use to understand module structure without reading every file. action=module path='src/' to zoom.

Parameters: `action`, `format`, `path`, `root`

## `ctx_artifacts`

Context artifact registry + BM25 index. Actions: list|status|index|reindex|search|remove.

Parameters: `action`*, `format`, `name`, `project_root`, `query`, `top_k`

## `ctx_benchmark`

Benchmark compression modes for a file or project.

Parameters: `action`, `format`, `path`*

## `ctx_cache`

Cache ops: status|clear|invalidate.

Parameters: `action`*, `path`

## `ctx_call`

Invoke any non-core lean-ctx tool by name.
Categories: arch, debug, memory, batch, agent, util. Find exact names with ctx_discover_tools (query=keyword; empty query lists all).

Parameters: `arguments`, `name`*

## `ctx_callgraph`

Callers/callees analysis — who calls a function and what it calls.
action=callers symbol='fn' returns every call site with file:line.
For END-TO-END flow tracing (how does X reach Y), use ctx_compose FIRST
— one call returns the path + source. Use ctx_callgraph only when you need
exhaustive enumeration of ALL callers/callees for a single symbol.
action=trace from→to finds path between two symbols. depth=N for BFS depth.

Parameters: `action`, `depth`, `file`, `from`, `symbol`, `to`

## `ctx_checkpoint`

Local shadow git history of the agent's changes (separate from the user's .git).
actions: snapshot (record current state) | log (list checkpoints) | diff (vs a checkpoint) | restore (revert files).
Snapshot before+after a change to capture exactly what the LLM modified; diff/restore to review or roll back.
Never touches the user's repository.

Parameters: `action`, `from`, `limit`, `message`, `path`, `ref`, `to`

## `ctx_compile`

Context compilation (CFT). Builds minimal context package via greedy knapsack + Boltzmann view selection. Modes: handles|compressed|full.

Parameters: `budget`, `mode`

## `ctx_compose`

PRIMARY TOOL — call FIRST for understanding code, before editing, debugging, or
answering 'how does X work'. Pass a task/question or symbol names. returns ranked files with 
relevant symbol source inline grouped by file. Combines BM25 lexical + semantic + associative
retrieval + submodular optimization. Do NOT chain search→read→symbol — one compose
does it all. Do NOT Read files whose source compose already returned — it IS the source.
Fire independent ctx_read or ctx_compose calls for different areas in PARALLEL.

Parameters: `path`, `task`*

## `ctx_compress`

Context checkpoint: compresses read cache to free budget in long sessions
include_signatures=true (default) preserves API surface in compressed state.
Does not affect session state or knowledge—only the read cache compaction.

Parameters: `include_signatures`

## `ctx_compress_memory`

Compress a memory/config file (CLAUDE.md, .cursorrules) preserving code, URLs, paths. Creates .original.md backup.

Parameters: `path`*

## `ctx_context`

Session context overview — cached files, seen files, session state.

Parameters: _none_

## `ctx_control`

Universal context manipulation (Context Field Theory). Actions: exclude|include|pin|unpin|set_view|set_priority|mark_outdated|reset|list|history. Overlay-based, reversible, scoped.

Parameters: `action`*, `reason`, `scope`, `target`, `value`

## `ctx_cost`

Cost attribution (local-first). Actions: report|agent|tools|json|reset.

Parameters: `action`, `agent_id`, `limit`

## `ctx_dedup`

Cross-file dedup: analyze or apply shared block references.

Parameters: `action`

## `ctx_delta`

Incremental diff since last read — shows only changed lines after you edit.
Use INSTEAD of re-reading the whole file after modifications — saves 90%+ tokens
on unchanged content. Path must have a prior ctx_read in this session's cache.
For the full git diff against HEAD, use ctx_read(path, mode=diff) instead.

Parameters: `path`*

## `ctx_discover`

Find missed compression opportunities in shell history.

Parameters: `limit`

## `ctx_discover_tools`

Search available lean-ctx tools by keyword. Returns matching tool names and descriptions.

Parameters: `query`

## `ctx_edit`

Search-and-replace edit: old_string must be unique unless replace_all=true
create=true writes new files from new_string. TOCTOU-guarded with preimage hash verification.
backup creates .bak before modifying. Supports MD5/size/mtime pre-guards for race-free edits.

Parameters: `create`, `new_string`*, `old_string`, `path`*, `replace_all`

## `ctx_execute`

Run code in sandbox (11 languages) — use when compute beats shell glue.
action=code (default) for one-shot transform/math/generation; action=batch for parallel
multi-language scripts; action=file to process a project file (extension auto-detects
language). Pass intent to focus large output and save tokens. Prefer over ctx_shell when
logic, conditionals, multi-line scripts, or cross-language data munging — stdout-only,
no argv escaping. Languages: javascript, typescript, python, shell, ruby, go, rust, php,
perl, r, elixir.

Parameters: `action`, `code`, `intent`, `items`, `language`, `path`, `timeout`

## `ctx_expand`

Retrieve archived tool output by ID (e.g. id=@F1 from [Archived:ID] hints).
Use when you see an [Archived:ID] reference in tool output and need the full
content. Supports head/tail/search to filter lines. action=search_all across
all archives. action=list shows available archives. Zero-loss: original preserved.
For reading files, use ctx_read or ctx_compose instead.

Parameters: `action`, `end_line`, `head`, `id`, `json_keys`, `json_path`, `query`, `search`, `session_id`, `start_line`, `tail`

## `ctx_feedback`

Harness feedback for LLM output tokens/latency (local-first). Actions: record|report|json|reset|status.

Parameters: `action`, `agent_id`, `intent`, `latency_ms`, `limit`, `llm_input_tokens`, `llm_output_tokens`, `model`, `note`

## `ctx_fill`

Budget-aware context fill — auto-selects compression per file within token limit.

Parameters: `budget`*, `paths`*, `task`

## `ctx_gain`

Gain report (includes Wrapped via action=wrapped).

Parameters: `action`, `limit`, `model`, `period`

## `ctx_git_read`

Read a remote git repository via a cached shallow clone (not HTML scraping).
modes: overview (tree + README) | tree (file list) | read (a file) | grep (search).
Accepts repo URLs and GitHub/GitLab blob/tree links (ref + path auto-detected). https-only, SSRF-guarded, bounded.
Use instead of ctx_url_read when you need a whole repo's files/structure.

Parameters: `max_tokens`, `mode`, `path`, `query`, `ref`, `timeout_secs`, `url`*

## `ctx_glob`

Find files by glob pattern. Respects .gitignore;
supports multi-root via `paths` array. max_results=N sets limit.
For file content search, use ctx_search (pattern) or ctx_semantic_search (meaning).

Parameters: `ignore_gitignore`, `max_results`, `path`, `paths`, `pattern`*

## `ctx_graph`

Code graph queries — find usages, relationships, and dependency chains.
action=symbol path='file.rs::fnName' finds all usages of a symbol.
action=neighbors shows adjacent nodes; action=path from→to shows dependency
chains between files. action=diff since=HEAD~1 for git change impact.
For understanding code end-to-end, use ctx_compose FIRST. Use ctx_graph for
targeted structural queries the graph index can answer directly.

Parameters: `action`*, `depth`, `format`, `kind`, `path`, `project_root`, `since`, `to`

## `ctx_handoff`

Context Ledger Protocol (hashed, deterministic, local-first). Actions: create|show|list|pull|clear|export|import.

Parameters: `action`, `apply_knowledge`, `apply_session`, `apply_workflow`, `filename`, `format`, `path`, `paths`, `privacy`, `write`

## `ctx_heatmap`

File access heatmap — shows most frequently accessed files.

Parameters: `action`, `path`

## `ctx_impact`

Change impact: action=analyze path='file.rs'→blast radius; depth=N; action=diff→git refs
action=chain from→to→dependency path. depth controls traversal (default 5).
Use before refactoring to assess risk. path can be file path or type/class name.

Parameters: `action`, `depth`, `format`, `path`, `root`

## `ctx_index`

Index orchestration. Actions: status|build|build-full.

Parameters: `action`*, `project_root`

## `ctx_intent`

Structured intent input (optional) — submit compact JSON or short text; server also infers intents automatically from tool calls.

Parameters: `project_root`, `query`*

## `ctx_knowledge`

Persistent memory across sessions — remember decisions, patterns, and facts.
action=remember saves a fact; action=recall query='X' retrieves it.
Use to persist architecture decisions, gotchas, and patterns for future sessions.
action=gotcha trigger='X' resolution='Y' for known pitfalls.
mode=semantic|exact for recall. category groups related facts.

Parameters: `action`*, `as_of`, `category`, `confidence`, `examples`, `key`, `mode`, `pattern_type`, `query`, `resolution`, `severity`, `trigger`, `value`

## `ctx_ledger`

Context ledger ops: status|reset|evict. Manages persistent context pressure.

Parameters: `action`*, `targets`

## `ctx_load_tools`

Load/unload specialized tool categories on demand. Categories: arch, debug, memory, metrics, session. Core is always loaded.

Parameters: `action`*, `category`

## `ctx_metrics`

Session token stats, cache rates, per-tool savings.

Parameters: _none_

## `ctx_multi_read`

Batch-read multiple files in one call — more token-efficient than N sequential
ctx_read calls. paths=['a.rs','b.rs'] reads them all at once.
mode=full for files you edit; mode=auto for general reading (compressed).
Use when you need the content of several files. For understanding code logic,
use ctx_compose FIRST — it returns relevant symbol source grouped by file.

Parameters: `fresh`, `mode`, `paths`*

## `ctx_multi_repo`

Multi-repo management: add/remove roots, cross-repo search with Reciprocal Rank Fusion (RRF). Enables searching across multiple project directories simultaneously.

Parameters: `action`*, `alias`, `max_results`, `path`, `query`, `roots`

## `ctx_outline`

File symbols: path='file.rs'->signatures; kind=fn|struct|class|all filter
Lists all named symbols in a file with signatures and line numbers.
Generated via tree-sitter extraction of fn/struct/class/trait declarations.

Parameters: `kind`, `path`*

## `ctx_overview`

Task-relevant project map — use at session start to orient before diving into code.
task='your goal' scopes files/modules by relevance (PageRank on symbol graph).
For deeper code understanding, use ctx_compose instead — returns source + flow
in one call. ctx_overview is lighter: high-level structure only, no source body.

Parameters: `path`, `task`

## `ctx_pack`

Context Package Manager. Actions: pr (PR context), create (build package from project), list, info, remove, install, export, import, auto_load, summary.

Parameters: `action`*, `apply`, `author`, `base`, `depth`, `description`, `diff`, `enable`, `file`, `format`, `layers`, `level`, `name`, `project_root`, `scope`, `tags`, `version`

## `ctx_package`

Save or resume portable context packages — self-contained JSON bundles with session state, summaries, and knowledge. Use to hand off context between agents, persist session snapshots for later, or onboard a new agent into a previous session's context. Actions: save (export current session), resume (import from a package file), list (show saved packages), info (inspect a package without importing).

Parameters: `action`, `description`, `path`

## `ctx_plan`

Context planning (CFT). Computes optimal context plan with Phi scoring, budget allocation, and policy-driven view selection.

Parameters: `budget`, `profile`, `task`*

## `ctx_plugins`

Plugin management. Actions: list (show installed plugins), enable (activate a plugin), disable (deactivate a plugin), info (show plugin details), hooks (list available hook points).

Parameters: `action`*, `name`

## `ctx_prefetch`

Predictive prefetch — prewarm cache for blast radius files (graph + task signals) within budgets.

Parameters: `budget_tokens`, `changed_files`, `max_files`, `root`, `task`

## `ctx_preload`

Proactive context loader — caches task-relevant files, returns L-curve-optimized summary (~50-100 tokens vs ~5000 for individual reads).

Parameters: `path`, `task`*

## `ctx_proof`

Export a machine-readable ContextProofV1 (Verifier + SLO + Pipeline + Provenance). Writes to .lean-ctx/proofs/ by default.

Parameters: `action`*, `filename`, `format`, `max_evidence`, `max_ledger_files`, `project_root`, `write`

## `ctx_provider`

External context providers (GitHub, GitLab, Jira, Postgres, MCP, custom REST).

Parameters: `action`*, `iid`, `labels`, `limit`, `mode`, `provider`, `resource`, `state`, `status`

## `ctx_radar`

Full context budget breakdown: system prompt, messages, tools, reads, shell — all tracked token usage.

Parameters: `format`

## `ctx_read`

Read source files. mode is REQUIRED — choose by intent:
full=verbatim (edit-ready, use before Edit), signatures=API surface only,
map=structural overview of large files, auto=smart (learns from task and
session context, use for orientation), diff=git delta, lines:N-M=window.
fresh=true bypasses cache.
For understanding code or finding answers, use ctx_compose FIRST instead.

Parameters: `aggressiveness`, `fresh`, `limit`, `mode`, `offset`, `path`*, `protect`, `start_line`

## `ctx_refactor`

LSP/IDE refactoring. action=one pipe-delimited value below. Reads (references/definition/implementations/declaration/type_hierarchy/symbols_overview/inspections) need a language server or the JetBrains backend. Symbol edits (replace/insert_before/insert_after_symbol) are name_path-addressed, IDE-first with a lossless headless fallback. Two-Phase ops (rename/move/safe_delete/inline _preview+_apply) need a JetBrains IDE (else BACKEND_REQUIRED) with a stateless plan_hash TOCTOU guard. rename/move/safe_delete block conflicts unless force=true; inline cannot be forced (→ UNSUPPORTED). reformat is Single-Phase, by name_path | path | path+line.

Parameters: `action`*, `column`, `direction`, `end_line`, `expected_hash`, `force`, `keep_definition`, `line`, `mode`, `name_path`, `new_body`, `new_name`, `optimize_imports`, `path`, `plan_hash`, `propagate`, `scope`, `search_comments`, `search_text_occurrences`, `target_parent`, `target_path`, `text`

## `ctx_repomap`

PageRank symbol map: focus_files=['path/*.rs'] boosts areas; max_tokens controls size (default 2048)
Shows structurally important symbols ranked by PageRank and session relevance.
Use for codebase-wide orientation; for task-scoped view use ctx_overview.

Parameters: `focus_files`, `max_tokens`, `path`

## `ctx_response`

Compress LLM response text (structural de-duplication).

Parameters: `text`*

## `ctx_retrieve`

Retrieve original uncompressed content from the session cache (CCR). Use when a compressed ctx_read output is insufficient.

Parameters: `path`*, `query`

## `ctx_review`

Automated code review: combines impact analysis, caller tracking, and test discovery. Actions: review (single file), diff-review (from git diff), checklist (structured review questions).

Parameters: `action`*, `depth`, `path`

## `ctx_routes`

HTTP routes: method=GET|POST filter; path='/api' prefix; auto-detects frameworks
Extracts endpoints from: Express, Flask, FastAPI, Actix, Spring, Rails, Next.js.
Use to discover API surface without reading route definition files.

Parameters: `method`, `path`

## `ctx_rules`

Cross-agent rules governance (ContextOps). Actions: sync (distribute rules to agents), diff (show drift), lint (check consistency), status (show sync state), init (create central config).

Parameters: `action`*, `agent`

## `ctx_search`

Regex pattern search — use when you know the exact pattern. For understanding code or
finding answers, use ctx_compose FIRST (one call replaces search+read+symbol chains).
pattern required; include='*.rs'; path scopes; max_results=N (default 20).
paths=['dir1','dir2'] for multi-root. ignore_gitignore bypasses .gitignore (needs role).

Parameters: `ignore_gitignore`, `include`, `max_results`, `path`, `paths`, `pattern`*

## `ctx_semantic_search`

Search code by MEANING (BM25+embeddings) — use when you know the concept but not the exact
symbol name. query='user auth' finds relevant code even with no keyword match.
Different from ctx_search (regex): use ctx_search for exact patterns, this for
fuzzy/conceptual. For understanding code end-to-end, use ctx_compose FIRST.
find_related(file_path, line) for context neighbors. mode=bm25|dense|hybrid.

Parameters: `action`, `file_path`, `languages`, `line`, `mode`, `path`, `path_glob`, `query`*, `top_k`

## `ctx_session`

Cross-session memory: action=task/finding/decision persists; load session_id=X resumes
Use at session end to persist progress; at start to restore previous work.
action=status for current snapshot; action=save commits state; action=reset clears.

Parameters: `action`*, `session_id`, `value`

## `ctx_share`

Share cached file contexts between agents. Actions: push (share files from your cache to another agent), pull (receive files shared by other agents), list (show all shared contexts), clear (remove your shared contexts).

Parameters: `action`*, `message`, `paths`, `to_agent`

## `ctx_shell`

Run shell commands with automatic output compression (~95 patterns).
Optimized for build/test/log output (cargo, npm, pytest, go test).
raw=true disables compression for verbatim output. Lossless for errors
and exit codes — [exit:N] footer for failure codes. cwd persists.

Parameters: `command`*, `cwd`, `env`, `raw`

## `ctx_skillify`

Codify recurring patterns from this project's session diary + knowledge into versioned, git-committable .cursor/rules/skillify-*.mdc files. Actions: mine (distill & write/merge rules), list (show generated rules), status (config + counts), promote (copy a project rule to ~/.cursor/rules). Precision-biased; only acts when invoked; re-runs are idempotent.

Parameters: `action`, `slug`

## `ctx_smart_read`

Auto-select optimal read mode for a file.

Parameters: `path`*

## `ctx_smells`

Code smell detection. Actions: scan|summary|rules|file.

Parameters: `action`, `format`, `path`, `root`, `rule`

## `ctx_summary`

Record and recall AI session summaries — compact, semantically-recallable digests of what was done (task, files, decisions, next steps). Actions: recall (find past summaries by query; semantic when embeddings are warm, else lexical), record (snapshot the current session now), list (recent summaries). Summaries are also captured automatically on the checkpoint cadence.

Parameters: `action`, `query`, `top_k`

## `ctx_symbol`

Get ONE symbol's body by name — exact, AST-precise (tree-sitter index). Use AFTER
ctx_compose gave you the overview and you need a specific symbol's full body.
For multiple symbols or understanding an area, use ctx_compose FIRST (returns
all relevant symbols grouped by file in one call). name='fnName' returns code block.
file='path.rs' narrows; kind='fn'|'struct'|'class'|'trait'|'enum' disambiguates.

Parameters: `file`, `kind`, `name`*

## `ctx_task`

Multi-agent task orchestration. Actions: create|update|list|get|cancel|message|info.

Parameters: `action`*, `description`, `message`, `state`, `task_id`, `to_agent`

## `ctx_tools`

Gateway to downstream MCP servers — unlimited external tools at ~constant context cost.
actions: find (query → top-N relevant tools as ChoiceCards) | call (proxy a `server::tool`) | list (servers+counts) | refresh.
Use find to discover, then call the chosen `server::tool`. Off by default ([gateway] config).

Parameters: `action`, `arguments`, `query`, `tool`

## `ctx_transcript_compact`

Compact an OpenAI-format message array deterministically: keep system + a fresh tail verbatim, replace older turns with a recoverable summary, and offload the raw turns into lean-ctx session memory (indexed for ctx_search/ctx_knowledge recall). Built for the Hermes context-engine plugin. Returns JSON {messages, stats}; tool_call/tool_result pairs are never split.

Parameters: `focus_topic`, `fresh_tail_tokens`, `messages`*, `protect_min_messages`

## `ctx_tree`

Directory tree with file counts per directory. depth=N (default 3);
show_hidden for dotfiles; paths for multi-root.
respect_gitignore filters ignored files (default true).

Parameters: `depth`, `path`, `paths`, `respect_gitignore`, `show_hidden`

## `ctx_url_read`

Fetch URL: pages→Markdown; PDF→text; YouTube→transcript; mode=auto best per type
mode=facts|quotes for research (claims+confidence). query='topic' to focus extraction.
GitHub blob/raw URLs auto-resolve to raw file. SSRF-guarded (no private IPs). max_tokens=6000.

Parameters: `max_items`, `max_tokens`, `mode`, `query`, `timeout_secs`, `url`*

## `ctx_verify`

Verification observability. Actions: stats (tool call statistics), proof|v2 (ContextProofV2 claim-based verification with Lean4 proofs).

Parameters: `action`, `format`

## `ctx_workflow`

Workflow rails (state machine + evidence). Actions: start|status|transition|complete|evidence_add|evidence_list|stop.

Parameters: `action`, `key`, `name`, `spec`, `to`, `value`

## `shell`

Shell command with auto-compression (~95 patterns). Alias for ctx_shell.
Output is compressed for token savings. For verbatim output pass raw=true.

Parameters: `command`*, `cwd`

