# Provider Framework Contract v1

**Status**: Stable  
**Version**: `PROVIDER_FRAMEWORK_V1_SCHEMA_VERSION = 1`  
**Runtime source**: `rust/src/core/providers/`

## Purpose

Provides structured access to external context sources through the MCP tool interface.
All provider data is a **first-class citizen**: it flows through the full consolidation pipeline
into BM25 index, Graph index, Knowledge facts, and Session cache.

## Architecture

```
ContextProvider::execute()
    → ProviderResult
    → consolidation::consolidate()
    → ConsolidationArtifacts { bm25_chunks, edges, facts, cache_entries }
    → apply_artifacts_to_stores() [background thread]
        → BM25Index::ingest()       — searchable via ctx_semantic_search
        → GraphIndex::merge_edges() — cross-source hints in ctx_read
        → Knowledge::remember()     — recallable via ctx_knowledge
        → SessionCache::set()       — fast re-reads
```

### Built-in Providers

| Provider | Auto-activates when | Resources |
|---|---|---|
| GitHub | `GITHUB_TOKEN` set | issues, pull_requests, actions |
| GitLab | `GITLAB_TOKEN` set | issues, merge_requests, pipelines |
| Jira | `JIRA_TOKEN` set | issues, sprints, projects |
| PostgreSQL | `DATABASE_URL` set | tables, schemas, queries |

### Config-based Providers

Custom REST APIs via TOML/JSON in `~/.config/lean-ctx/providers/` or `.lean-ctx/providers/`.
Supports 6 auth methods (bearer, API key, basic, header, query param, none).

### MCP Bridge Providers

External MCP servers connected via `[providers.mcp_bridges.<name>]` config.
Each bridge registers with unique ID `mcp:<name>`. Supports:
- HTTP transport (`url = "http://..."`)
- Stdio transport (`command = "npx"`, `args = ["-y", "@mcp/server"]`)
- Actions: `resources` (list), `read_resource` (fetch single), `tools` (list)

## ctx_provider Actions

| Action | Parameters | Description |
|---|---|---|
| `gitlab_issues` | state, labels, limit | List issues (sorted by updated_at desc) |
| `gitlab_issue` | iid | Show single issue with description |
| `gitlab_mrs` | state, limit | List merge requests |
| `gitlab_pipelines` | status, limit | List pipelines |
| `mcp_resources` | — | List all resources from configured MCP bridges |

## Configuration

Token resolution order:
1. `LEAN_CTX_GITLAB_TOKEN`
2. `GITLAB_TOKEN`
3. `CI_JOB_TOKEN`

Host resolution:
1. `GITLAB_HOST`
2. `CI_SERVER_HOST`
3. Default: `gitlab.com`

Project path resolution:
1. `CI_PROJECT_PATH`
2. Auto-detect from `git remote get-url origin`

## ProviderResult Schema

```rust
struct ProviderResult {
    provider: String,       // "gitlab"
    resource_type: String,  // "issues", "merge_requests", "pipelines"
    items: Vec<ProviderItem>,
    total_count: Option<usize>,
    truncated: bool,
}

struct ProviderItem {
    id: String,
    title: String,
    state: Option<String>,
    author: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    url: Option<String>,
    labels: Vec<String>,
    body: Option<String>,
}
```

## Caching & Indexing

- **Session cache**: TTL-based in-memory cache (120 seconds default). Cache key includes provider, resource type, project, filters
- **BM25 index**: External chunks indexed with `ChunkKind` metadata (Issue, PullRequest, DbSchema, etc.)
- **Graph index**: Cross-source edges link external URIs to code files (e.g. issue → `src/auth.rs`)
- **Knowledge facts**: Extracted categories: `known_bugs`, `known_features`, `recent_changes`, `data_model`, `documentation`, `file_mentions`
- **`providers.auto_index`**: Controls background indexing (default: `true`)

## Security

- All provider outputs pass through `redact_text_if_enabled`
- CI job logs pass through secret scanner before delivery
- Tokens never appear in tool output
- MCP bridges: optional `auth_env` field for token injection from env vars

## Shell Compression (`glab` / `gh` CLI)

Patterns for CLI output:
- `glab`/`gh` issue list/view, MR/PR list/view, CI/actions status
- Compression follows pattern-based structure

## Context IR Integration

Provider outputs are tracked as `ContextIrSourceKindV1::Provider` in the evidence ledger, enabling:
- Provenance tracking (which provider data informed a decision)
- Replay verification
- Token attribution

## Diagnostics

`lean-ctx doctor` validates:
- Provider env vars (GITHUB_TOKEN, GITLAB_TOKEN, etc.)
- MCP bridge URLs (reachable, configured)
- `auto_index` status (warns if `false`)
