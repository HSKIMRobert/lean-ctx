# Memory Boundary Contract v1

**Status**: Stable  
**Version**: `MEMORY_BOUNDARY_V1_SCHEMA_VERSION = 1`  
**Runtime source**: `rust/src/core/memory_boundary.rs`

## Purpose

Prevents cross-project memory leakage. By default, all knowledge facts, sessions, and gotchas are scoped to their originating project. Cross-project access requires explicit opt-in via role policy.

## FactPrivacy

Every `KnowledgeFact` carries a `privacy` field:

```rust
enum FactPrivacy {
    ProjectOnly,    // default — only visible within originating project
    LinkedProjects, // visible to projects listed in .leanctx.json linkedProjects
    Team,           // visible to all projects on the team server
}
```

## BoundaryPolicy

Controls cross-project access at the policy level:

```rust
struct BoundaryPolicy {
    cross_project_search: bool,  // default: false
    cross_project_import: bool,  // default: false
    audit_cross_access: bool,    // default: true
}
```

## Enforcement Points

| Tool / Action | Default Behavior | Cross-Project Behavior |
|---|---|---|
| `ctx_knowledge action=search` | Scoped to current `project_hash` | Requires `IoPolicy.allow_cross_project_search = true` |
| `ctx_knowledge action=recall` | Loads from current project | No cross-project access |
| `ctx_handoff action=import` | Identity check enforced | Mismatch = blocked + audit event |
| Session `load_latest` | Scoped to project root | No global fallback |
| Universal gotchas | Shared by design | `universal-gotchas.json` exempt from boundary |

## Audit

When `audit_cross_access` is true (default), cross-project access attempts are logged to:

```
~/.lean-ctx/audit/cross-project.jsonl
```

Each event contains:

```json
{
  "timestamp": "2026-05-03T09:43:12Z",
  "event_type": "search",
  "source_project_hash": "abc123",
  "target_project_hash": "def456",
  "tool": "ctx_knowledge",
  "action": "search",
  "facts_accessed": 3,
  "allowed": false,
  "policy_reason": "cross_project_search disabled"
}
```

## Role Integration

The `IoPolicy` on each role controls cross-project access:

```toml
[io]
allow_cross_project_search = false  # default for all roles
```

Admin roles can override:

```toml
[io]
allow_cross_project_search = true
```

## Migration

Existing `KnowledgeFact` entries without a `privacy` field default to `ProjectOnly` via `#[serde(default)]`. No migration required.

## Drift Gate

`rust/tests/contracts_md_up_to_date.rs` verifies that `MEMORY_BOUNDARY_V1_SCHEMA_VERSION` in `contracts.rs` matches the documented version in `CONTRACTS.md`.
