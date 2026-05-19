# LeanCTX Protocol Family & Contracts (v1)

LeanCTX is infrastructure. Contracts are the stable promises that client integrations, CI gates, proof artifacts, and future plugins rely on.

## Architecture positioning

```
MCP = how agents call LeanCTX (external interoperability)
LCP = how LeanCTX understands, transforms, and governs context (internal semantics)
```

MCP is the transport. The contracts below define what flows through it.

## Versioning rules

- **Schema versions are integers** (`schema_version` / `contract_version`).
- **Breaking change** => bump the corresponding version and add migration notes.
  - Examples: removing fields, changing field types, changing required fields, changing error semantics/status codes.
- **Non-breaking change** => keep version, document additive changes.
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

---

## Core Context Contracts

Foundational representations that all other contracts build upon.

### Context IR v1 (Intermediate Representation)

The canonical representation of all context flowing through LeanCTX. Every tool call is recorded with source, lineage, tokens, compression ratio, and safety metadata.

- **Doc**: `docs/contracts/context-ir-v1.md`
- **Runtime source**: `rust/src/core/context_ir.rs`
- **Surface**: Recorded in hot-path after every `call_tool`; exported via `ctx_proof`; persisted to `~/.lean-ctx/context_ir_v1.json`

### Context Proof v1 (Verification Artifacts)

Cryptographic proofs that document what context was produced, how it was compressed, and whether it's reproducible.

- **Runtime source**: `rust/src/core/context_proof.rs`
- **Surface**: `ctx_proof` tool; exports to `project/.lean-ctx/proofs/`

### Verification Observability v1

Runtime observability for output verification (compression safety checks).

- **Runtime source**: `rust/src/core/verification_observability.rs`
- **Surface**: Verify footer in tool outputs when profile-enabled

---

## Runtime Contracts

Govern how the runtime processes, budgets, and degrades context.

### Degradation Policy v1 (Budgets/SLOs)

- **Doc**: `docs/contracts/degradation-policy-v1.md`
- **Runtime source**: `rust/src/core/degradation_policy.rs`
- **Surface**: Enforced at tool-call boundary when enabled

### Workflow Evidence Ledger v1

- **Doc**: `docs/contracts/workflow-evidence-ledger-v1.md`
- **Runtime source**: `rust/src/core/evidence_ledger.rs`
- **Surface**: `ctx_workflow` evidence-gated transitions + automatic tool receipts

### Autonomy Drivers v1

- **Doc**: `docs/contracts/autonomy-drivers-v1.md`
- **Runtime source**: `rust/src/core/autonomy_drivers.rs` + `rust/src/tools/autonomy.rs`
- **Surface**: Deterministic driver planner + bounded driver reports; proof export via `ctx_proof`

### Intent Route v1 (Orchestration Routing)

- **Doc**: `docs/contracts/intent-route-v1.md`
- **Runtime source**: `rust/src/core/intent_router.rs`
- **Surface**: `ctx_intent` with `format=json` returns `IntentRouteV1`

### Tokenizer-aware Translation Driver v1

- **Doc**: `docs/contracts/tokenizer-translation-driver-v1.md`
- **Runtime source**: `rust/src/core/tokenizer_translation_driver.rs`
- **Surface**: Deterministic ruleset selection (model_key -> ruleset) + bounded translation

### Attention-aware Layout Driver v1

- **Doc**: `docs/contracts/attention-layout-driver-v1.md`
- **Runtime source**: `rust/src/core/attention_layout_driver.rs`
- **Surface**: Deterministic reorder for delivery surfaces when profile-enabled

---

## Memory & Collaboration Contracts

Define how context persists, transfers between agents, and crosses boundaries.

### CCP Session Bundle v1

- **Doc**: `docs/contracts/ccp-session-bundle-v1.md`
- **Runtime source**: `rust/src/core/ccp_session_bundle.rs` + `rust/src/core/session.rs`
- **Surface**: `ctx_session action=export|import` (redacted-by-default, bounded, replayable)

### Knowledge Policy v1

- **Doc**: `docs/contracts/knowledge-policy-contract-v1.md`
- **Runtime source**: `rust/src/core/memory_policy.rs` + `rust/src/core/knowledge.rs`
- **Surface**: `ctx_knowledge action=policy value=show|validate`

### Graph Reproducibility v1

- **Doc**: `docs/contracts/graph-reproducibility-contract-v1.md`
- **Runtime source**: `rust/src/core/property_graph/*`
- **Surface**: `ctx_impact` / `ctx_architecture` with `format=json`

### A2A Contract v1 (Multi-Agent)

- **Doc**: `docs/contracts/a2a-contract-v1.md`
- **Runtime source**: `rust/src/core/agents.rs` + `rust/src/core/a2a/*`
- **Surface**: `ctx_agent`, `ctx_task`, rate limiting, cost attribution

### Handoff Transfer Bundle v1

- **Doc**: `docs/contracts/handoff-transfer-bundle-v1.md`
- **Runtime source**: `rust/src/core/handoff_transfer_bundle.rs`
- **Surface**: `ctx_handoff action=export|import` (redacted-by-default, bounded, identity-aware)

### Memory Boundary v1

- **Doc**: `docs/contracts/memory-boundary-contract-v1.md`
- **Runtime source**: `rust/src/core/memory_boundary.rs`
- **Surface**: `FactPrivacy` scoping, cross-project gates, audit events

### Gotchas/Reminders v1

- **Doc**: `docs/contracts/gotchas-reminders-contract-v1.md`
- **Runtime source**: `rust/src/core/gotcha_tracker/model.rs`
- **Surface**: Time-bounded reminders with provenance and decay

---

## Extension Contracts

Interfaces for external integrations and future plugin system.

### Provider Framework v1 (Context I/O)

- **Doc**: `docs/contracts/provider-framework-contract-v1.md`
- **Runtime source**: `rust/src/core/providers/` + `rust/src/tools/ctx_provider.rs`
- **Surface**: `ctx_provider` tool (GitLab issues, MRs, pipelines); TTL-based cache; redaction on all outputs
- **Future**: This contract defines the shape for third-party Context Provider plugins

### CompressionPattern (planned v1)

- **Status**: Interface extracted from `rust/src/core/patterns/mod.rs`
- **Future**: Plugin-loadable compression patterns for proprietary CLI tools

### LcpTool (planned v1)

- **Status**: Mirrors existing `McpTool` trait in `rust/src/server/tool_trait.rs`
- **Future**: Plugin-registered tools that inherit the full runtime pipeline

---

## Transport Contracts

Define how LeanCTX communicates with the outside world.

### MCP Manifest v1 (Tool Inventory)

- **Artifact**: `website/generated/mcp-tools.json`
- **Schema**: `schema_version` + normalized tool entries (`name`, `description`, `input_schema`, `schema_md5`)
- **Runtime source**: `rust/src/core/mcp_manifest.rs`

### HTTP MCP v1

- **Doc**: `docs/contracts/http-mcp-contract-v1.md`
- **Stable endpoints**: `/health`, `/v1/manifest`, `/v1/tools`, `/v1/tools/call`, `/v1/events`, `/v1/context/summary`
- **Event schema**: `ContextEventV1` with `version`, `parentId`, `consistencyLevel`
- **Typed errors**: JSON `error_code` + `error`

### Team Server v1

- **Doc**: `docs/contracts/team-server-contract-v1.md`
- **Workspaces**: `x-leanctx-workspace` header + `workspaceId` body + deterministic fallback
- **Audit log**: JSONL with `argumentsMd5` only (no raw args)

---

## Compatibility matrix

| Integration | Transport | Contracts relied on | Setup |
|---|---|---|---|
| Cursor | MCP (stdio) + Shell Hook | MCP manifest v1 + tool schemas + shell patterns | `lean-ctx setup` |
| Claude Code | MCP (stdio) + Shell Hook | MCP manifest v1 + tool schemas + shell patterns | `lean-ctx init --agent claude` |
| GitHub Copilot | MCP (stdio) + Shell Hook | MCP manifest v1 + tool schemas | `lean-ctx init --agent copilot` |
| Remote agents | HTTP | HTTP MCP v1 + typed errors | `lean-ctx serve` |
| Teams | HTTP | Team Server v1 + audit log | `lean-ctx team serve` |
| Future plugins | In-process / Subprocess | Provider v1 + CompressionPattern v1 + LcpTool v1 | `~/.config/lean-ctx/plugins/` |
