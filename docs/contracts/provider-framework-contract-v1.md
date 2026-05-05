# Provider Framework Contract v1

**Status**: Stable  
**Version**: `PROVIDER_FRAMEWORK_V1_SCHEMA_VERSION = 1`  
**Runtime source**: `rust/src/core/providers/`

## Purpose

Provides structured access to external context sources (GitLab issues, MRs, pipelines) through the MCP tool interface, with caching, redaction, and Context IR tracking.

## Architecture

Two-tiered approach:
1. **MCP Provider Tool** (`ctx_provider`): REST API client for GitLab v4
2. **Shell Compression** (`glab.rs`): Pattern-based compression for `glab` CLI output

## ctx_provider Actions

| Action | Parameters | Description |
|---|---|---|
| `gitlab_issues` | state, labels, limit | List issues (sorted by updated_at desc) |
| `gitlab_issue` | iid | Show single issue with description |
| `gitlab_mrs` | state, limit | List merge requests |
| `gitlab_pipelines` | status, limit | List pipelines |

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

## Caching

- TTL-based in-memory cache (120 seconds default)
- Cache key includes: provider, resource type, project, filters
- Cache entries auto-expire

## Security

- All provider outputs pass through `redact_text_if_enabled`
- CI job logs pass through secret scanner before delivery
- Tokens never appear in tool output

## Shell Compression (`glab` CLI)

Patterns for `glab` CLI output:
- `glab issue list` / `glab issue view`
- `glab mr list` / `glab mr view`
- `glab ci status` / `glab ci list` / `glab ci view`

Compression follows the same structure as `gh.rs` patterns.

## Context IR Integration

Provider outputs are tracked as `ContextIrSourceKindV1::Provider` in the evidence ledger, enabling:
- Provenance tracking (which GitLab data informed a decision)
- Replay verification
- Token attribution
