# System Inventory — Phase 0 Architecture Freeze

## Date: 2026-08-05

## Purpose
Maps existing codebase modules to the target architecture domains.
Establishes data ownership per domain for the rebuild.

---

## Current Workspace Structure (OSS Repo)

| Crate | LOC (approx) | Target Domain |
|-------|--------------|---------------|
| `lean-ctx` (main binary) | ~550k | Runtime (stays OSS) |
| `lean-ctx-ocla` | ~1.2k | Runtime OCLA Layer (stays OSS) |
| `lean-ctx-protocol` | ~420 | Protocol Wire Types (NEW, OSS) |
| `lean-ctx-sdk` | ~2k | SDK (stays OSS) |
| `grammar-addons/lua` | ~200 | Runtime Addons (stays OSS) |

## Core Modules → Domain Mapping

### Stays in OSS Runtime (no changes needed for architecture)
| Module | Responsibility |
|--------|---------------|
| `cache/` | Response caching, compression |
| `shell_allowlist/` | Shell command safety |
| `signatures_ts/` | Tree-sitter AST parsing |
| `extractors/` | Content extraction |
| `extractive/` | Extractive compression |
| `terse/` | Terse output formatting |
| `input_filters/` | Input preprocessing |
| `session/` | Session state management |
| `knowledge/` | Local knowledge store |
| `bm25_index/` | BM25 search index |
| `graph_index/` | Graph-based search |
| `embeddings/` | Embedding generation |
| `providers/` | LLM provider abstraction |
| `config/` | Configuration loading |
| `git/` | Git integration |
| `plugins/` | Plugin system |
| `mcp_catalog/` | MCP tool registry |
| `web/` | Web interface |
| `agents/` | Agent bus coordination |
| `addons/` | Addon loading |
| `profiles/` | User profiles |
| `repomap/` | Repository mapping |
| `stats/` | Statistics |
| `patterns/` | Pattern matching |
| `context_kernel/` | Context engine core |
| `context_ledger/` | Context tracking |
| `context_os/` | Context OS abstraction |
| `context_package/` | Context packaging |
| `context_snapshot/` | Context snapshots |
| `contextops/` | Context operations |
| `deep_queries/` | Deep code queries |
| `import_resolver/` | Import resolution |
| `property_graph/` | Property graph |
| `graph_analysis/` | Graph analysis |
| `editor_registry/` | Editor integration |
| `code_health/` | Code health metrics |
| `codesign/` | macOS code signing |
| `workflow/` | Workflow engine |
| `skillify/` | Skill system |
| `buddy/` | Interactive assistant |
| `neural/` | Neural features |
| `locomo/` | Locomotion/navigation |
| `community/` | Community features |
| `godot/` | Game engine integration |

### Requires Refactoring for Architecture Boundary

| Module | Current | Target | Action |
|--------|---------|--------|--------|
| `ocla/` | Mixed OSS/Platform types | OSS: execution only | Extract protocol types → `lean-ctx-protocol` (DONE) |
| `ocla/builtin/experiment_runner.rs` | Decides + executes | OSS: executes only | Renamed to `experiment_executor.rs` (DONE) |
| `ocla/policy_bundle.rs` | Basic rules | OSS: enforces with classes | Extended with PolicyCriticality (DONE) |
| `ocla/types.rs` | All types mixed | OSS: runtime-only types | Marked PROTOCOL/RUNTIME (DONE) |
| `policy/` | Local enforcement | OSS: enforces signed policies | Needs Sidecar transport integration |
| `savings_ledger/` | Local savings tracking | OSS: emits SavingsObservationV1 | Needs protocol type emission |
| `eval_ab/` | Local A/B evaluation | OSS: executes assignments only | Align with experiment_executor |
| `billing/` | Plan/entitlement checks | OSS: entitlement (union) only | Separate from governance (intersection) |
| `gain/` | Cost/savings calculation | OSS: local estimates only | VerifiedSavings stays proprietary |
| `finops_export/` | FinOps reporting | OSS: local reports | Platform integration via Sidecar |
| `compliance_report/` | Compliance | OSS: local checks | Enterprise compliance via Platform |

---

## Data Ownership per Domain (Target Architecture)

### OSS Runtime (this repo) — OWNS:
- Local configuration
- Session state (volatile)
- Local cache (content-addressed)
- Local knowledge store
- BM25/Graph indices
- In-memory event ring-buffer (volatile)
- Gap-Journal (reserved 1MB)
- Unsigned local savings observations

### Proprietary Platform — OWNS:
- Organizations, Tenants, Billing Accounts
- Contracts, Pricing, Fee Schedules
- VerifiedSavingsV1 records
- Audit trail (tamper-evident)
- Policy definitions and signatures
- Experiment configurations and evaluations
- User identities, roles, permissions
- Connector credentials
- KMS keys, DEKs

### Sidecar — OWNS:
- Encrypted durable spool (disk buffer)
- DEK from KMS/Vault
- Signed policy cache
- Session credentials (ephemeral)

---

## Architecture Freeze Declaration

With 14 ADRs committed and this inventory complete:

> **The architecture is frozen as of 2026-08-05.**
> No structural changes to the domain model without a new ADR.
> Implementation proceeds according to Phase 1 → Phase 8 sequence.

### What is frozen:
- Domain boundaries (OSS vs. Proprietary)
- Data ownership per domain
- Communication pattern (Sidecar + Outbox)
- Type definitions in `lean-ctx-protocol`
- Key/Buffer ownership per hop
- Entitlement (union) vs. Governance (intersection)

### What is NOT frozen:
- Implementation details within domains
- Internal module organization within a domain
- Specific database schema (expand-and-contract allows evolution)
- Console UI design
