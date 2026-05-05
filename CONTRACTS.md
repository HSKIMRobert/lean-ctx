# LeanCTX Contracts & Versioning Policy (v1)

GitLab: `#2336`

LeanCTX is infrastructure. Contracts are the stable promises that client integrations, CI gates, and proof artifacts rely on.

## Versioning rules

- **Schema versions are integers** (`schema_version` / `contract_version`).
- **Breaking change** ⇒ bump the corresponding version and add migration notes.
  - Examples: removing fields, changing field types, changing required fields, changing error semantics/status codes.
- **Non-breaking change** ⇒ keep version, document additive changes.
  - Examples: adding optional fields, adding new tools, adding new docs pages.
- **Compatibility**:
  - Newer runtimes should be able to **read older artifacts** where possible (at least for proofs / observability).
  - If multiple versions are supported concurrently, support is **explicitly documented**.

## Current contract versions (SSOT, machine-checked)

<!-- leanctx-contracts-kv:begin -->
leanctx.contract.mcp_manifest.schema_version=1
leanctx.contract.context_proof_v1.schema_version=1
leanctx.contract.context_ir_v1.schema_version=1
leanctx.contract.intent_route_v1.schema_version=1
leanctx.contract.degradation_policy_v1.schema_version=1
leanctx.contract.workflow_evidence_ledger_v1.schema_version=1
leanctx.contract.autonomy_drivers_v1.schema_version=1
leanctx.contract.tokenizer_translation_driver_v1.schema_version=1
leanctx.contract.attention_layout_driver_v1.schema_version=1
leanctx.contract.verification_observability_v1.schema_version=1
leanctx.contract.handoff_ledger_v1.schema_version=1
leanctx.contract.handoff_transfer_bundle_v1.schema_version=1
leanctx.contract.ccp_session_bundle_v1.schema_version=1
leanctx.contract.knowledge_policy_v1.schema_version=1
leanctx.contract.graph_reproducibility_v1.schema_version=1
leanctx.contract.a2a_snapshot_v1.schema_version=1
leanctx.contract.memory_boundary_v1.schema_version=1
leanctx.contract.gotchas_reminders_v1.schema_version=1
leanctx.contract.provider_framework_v1.schema_version=1
leanctx.contract.http_mcp.contract_version=1
leanctx.contract.team_server.contract_version=1
<!-- leanctx-contracts-kv:end -->

## Contracts

### Tool inventory / MCP manifest

- **Artifact**: `website/generated/mcp-tools.json`
- **Schema**: `schema_version` + normalized tool entries (`name`, `description`, `input_schema`, `schema_md5`)
- **Runtime source**: `rust/src/core/mcp_manifest.rs`

### Intent Route v1 (Orchestration routing policy)

- **Doc**: `docs/contracts/intent-route-v1.md`
- **Runtime source**: `rust/src/core/intent_router.rs`
- **Surface**: `ctx_intent` with `format=json` returns `IntentRouteV1`

### Degradation Policy v1 (Budgets/SLOs)

- **Doc**: `docs/contracts/degradation-policy-v1.md`
- **Runtime source**: `rust/src/core/degradation_policy.rs`
- **Surface**: Enforced consistently at tool-call boundary (MCP/HTTP/Team) when enabled

### Workflow Evidence Ledger v1 (Workflows + Evidence)

- **Doc**: `docs/contracts/workflow-evidence-ledger-v1.md`
- **Runtime source**: `rust/src/core/evidence_ledger.rs`
- **Surface**: `ctx_workflow` evidence-gated transitions + automatic tool receipts

### Autonomy Drivers v1 (Autonomy)

- **Doc**: `docs/contracts/autonomy-drivers-v1.md`
- **Runtime source**: `rust/src/core/autonomy_drivers.rs` + `rust/src/tools/autonomy.rs`
- **Surface**: deterministic driver planner + bounded driver reports; proof export via `ctx_proof`

### Tokenizer-aware Translation Driver v1 (Cognition)

- **Doc**: `docs/contracts/tokenizer-translation-driver-v1.md`
- **Runtime source**: `rust/src/core/tokenizer_translation_driver.rs` + `rust/src/core/neural/token_optimizer.rs`
- **Surface**: deterministic ruleset selection (model_key → ruleset) + bounded translation; benchmark via `ctx_benchmark`

### Attention-aware Layout Driver v1 (Cognition)

- **Doc**: `docs/contracts/attention-layout-driver-v1.md`
- **Runtime source**: `rust/src/core/attention_layout_driver.rs` + `rust/src/core/neural/context_reorder.rs` + `rust/src/core/semantic_chunks.rs`
- **Surface**: deterministic reorder (chunks first, line-level fallback) for delivery surfaces (e.g. `ctx_read` full) when profile-enabled

### CCP Session Bundle v1 (Context Memory)

- **Doc**: `docs/contracts/ccp-session-bundle-v1.md`
- **Runtime source**: `rust/src/core/ccp_session_bundle.rs` + `rust/src/core/session.rs`
- **Surface**: `ctx_session action=export|import` (redacted-by-default, bounded, replayable)

### Knowledge Policy Contract v1 (Context Memory)

- **Doc**: `docs/contracts/knowledge-policy-contract-v1.md`
- **Runtime source**: `rust/src/core/memory_policy.rs` + `rust/src/core/knowledge.rs` + `rust/src/core/memory_lifecycle.rs`
- **Surface**: `ctx_knowledge action=policy value=show|validate` + deterministic enforcement in knowledge retrieval/actions

### Graph Reproducibility Contract v1 (Context Memory)

- **Doc**: `docs/contracts/graph-reproducibility-contract-v1.md`
- **Runtime source**: `rust/src/core/property_graph/*` + `rust/src/tools/ctx_impact.rs` + `rust/src/tools/ctx_architecture.rs`
- **Surface**: `ctx_impact` / `ctx_architecture` with `format=json` + architecture proof artifacts exported via `ctx_proof`

### A2A Contract v1 (Context Memory)

- **Doc**: `docs/contracts/a2a-contract-v1.md`
- **Runtime source**: `rust/src/core/agents.rs` + `rust/src/core/a2a/*` + `rust/src/tools/ctx_agent.rs` + `rust/src/tools/ctx_task.rs`
- **Surface**:
  - `ctx_agent` (privacy + TTL + bounded export snapshot v1)
  - `ctx_task` (task state machine + transitions)
  - Rate limiting (agent/tool/global) at MCP tool boundary
  - Cost attribution (cached tokens supported in store + reports)

### Handoff Transfer Bundle v1 (Context Memory / Delivery)

- **Doc**: `docs/contracts/handoff-transfer-bundle-v1.md`
- **Runtime source**: `rust/src/core/handoff_transfer_bundle.rs`
- **Surface**: `ctx_handoff action=export|import` (redacted-by-default, bounded, identity-aware)

### Provider Framework Contract v1 (Context I/O)

- **Doc**: `docs/contracts/provider-framework-contract-v1.md`
- **Runtime source**: `rust/src/core/providers/` + `rust/src/tools/ctx_provider.rs` + `rust/src/core/patterns/glab.rs`
- **Surface**:
  - `ctx_provider` tool with actions: `gitlab_issues`, `gitlab_issue`, `gitlab_mrs`, `gitlab_pipelines`
  - GitLab REST v4 via `ureq`, token from `GITLAB_TOKEN` / `LEAN_CTX_GITLAB_TOKEN` / `CI_JOB_TOKEN`
  - TTL-based provider cache (120s default)
  - Context IR integration via `ContextIrSourceKindV1::Provider`
  - `glab` CLI shell compression patterns (issue, mr, ci)
  - Redaction on all provider outputs

### Gotchas/Reminders Contract v1 (Context Memory)

- **Doc**: `docs/contracts/gotchas-reminders-contract-v1.md`
- **Runtime source**: `rust/src/core/gotcha_tracker/model.rs` + `rust/src/core/memory_policy.rs`
- **Surface**:
  - `Gotcha` struct with `ProvenanceRef` (kind, url, commit_hash, tool_call_id, session_id)
  - `expires_at` for time-bounded reminders, `decay_rate_override` per gotcha
  - `GotchaPolicy` in `MemoryPolicy` — `retrieval_budget_per_room`, `category_decay_overrides`
  - Unified schema in `granular.rs` — `policy` action added to enum

### Memory Boundary Contract v1 (Context Memory / Governance)

- **Doc**: `docs/contracts/memory-boundary-contract-v1.md`
- **Runtime source**: `rust/src/core/memory_boundary.rs` + `rust/src/core/knowledge.rs` + `rust/src/tools/ctx_knowledge.rs`
- **Surface**:
  - `FactPrivacy` enum (ProjectOnly, LinkedProjects, Team) on every `KnowledgeFact`
  - `BoundaryPolicy` (cross_project_search, cross_project_import, audit_cross_access)
  - `ctx_knowledge action=search` default-scoped to current project hash
  - Cross-project search requires explicit `allow_cross_project_search` in IoPolicy
  - `CrossProjectAuditEvent` logged to `audit/cross-project.jsonl`
  - Handoff import gate: identity mismatch enforced (not just warned)

### HTTP MCP Contract v1

- **Doc**: `docs/contracts/http-mcp-contract-v1.md`
- **Stable endpoints**: `/health`, `/v1/manifest`, `/v1/tools`, `/v1/tools/call`
- **Typed errors**: JSON `error_code` + `error`

### Team Server Contract v1

- **Doc**: `docs/contracts/team-server-contract-v1.md`
- **Workspaces**: `x-leanctx-workspace` header + `workspaceId` body + deterministic fallback
- **Audit log**: JSONL with `argumentsMd5` only (no raw args)

### Proof artifacts

- **ContextProofV1**: `rust/src/core/context_proof.rs` (`schema_version`)
- **ContextIrV1**: `docs/contracts/context-ir-v1.md` (`schema_version`)
- **DegradationPolicyV1**: `docs/contracts/degradation-policy-v1.md` (`schema_version`)
- **VerificationObservabilityV1**: `rust/src/core/verification_observability.rs` (`schema_version`)

### A2A handoff packages

- **HandoffLedgerV1**: `rust/src/core/handoff_ledger.rs` (`schema_version`)
- **HandoffTransferBundleV1**: `docs/contracts/handoff-transfer-bundle-v1.md` + `rust/src/core/handoff_transfer_bundle.rs` (`schema_version`)

## Compatibility matrix (integrations → contracts)

This matrix is intentionally phrased in terms of **LeanCTX contracts**, not external tool versions.

| Integration | Transport | Contracts relied on | Setup entrypoint |
|---|---|---|---|
| Cursor | MCP (stdio) + Shell Hook | MCP manifest v1 + tool schemas + shell hook patterns | `lean-ctx setup` |
| Claude Code | MCP (stdio) + Shell Hook | MCP manifest v1 + tool schemas + shell hook patterns | `lean-ctx init --agent claude` |
| GitHub Copilot | MCP (stdio) + Shell Hook | MCP manifest v1 + tool schemas | `lean-ctx init --agent copilot` |
| Remote agents / S2S | HTTP | HTTP MCP Contract v1 + typed errors | `lean-ctx serve ...` |
| Teams (multi-workspace) | HTTP | Team Server Contract v1 + audit log | `lean-ctx team serve --config team.json` |

