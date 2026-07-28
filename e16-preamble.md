# E16 Preamble — A2A Remote + Agent Fabric

## Project
Rust workspace at `rust/`. Binary: `lean-ctx`. 554k LOC, 9352 tests.

## Build & Quality Gate
```bash
cd rust
cargo fmt
cargo clippy --lib -- -D warnings
cargo test --lib 2>&1 | tail -5
```

## Code Style
- No `use super::*` — explicit imports only
- No `#[serde(deny_unknown_fields)]` on extensible structs
- `#[serde(default, skip_serializing_if = "Option::is_none")]` for optional fields
- `#[allow(dead_code)]` with comment if not yet wired
- Tests in `#[cfg(test)] mod tests {}` at bottom

## Pipeline-Breaking Patterns to Avoid
1. If you ADD a field to a struct: search ALL files for struct literal construction
2. If you ADD a tool to registry.rs: update the SSOT count in `server/mod.rs`
3. If you MODIFY types.rs or wire.rs: OpenAPI snapshot needs regeneration
4. Run `cargo clippy --lib -- -D warnings` BEFORE committing
5. NEVER use `deny_unknown_fields` on extensible serde structs
6. If you add new fields to existing structs, use `#[serde(default)]` and update ALL test helpers

## Key Imports
```rust
use crate::core::ocla::types::{OclaResult, OclaError, OclaRequestContext, OclaCapability, OclaCapabilityKind, AgentEnvelope};
use crate::core::ocla::traits::{OclaService, AgentGateway, ExperimentRunner};
use crate::core::ocla::registry::OclaRegistry;
use crate::core::a2a::remote_transport::{RemoteTransport, RemoteTransportConfig};
use crate::core::a2a::budget_cascade::{BudgetAllocation, CascadedBudget, CascadeError};
use crate::core::a2a::message::{MessagePriority, PrivacyLevel};
use crate::core::ocla_bus::{self, OclaEvent};
```

## A2A Module Structure
```
src/core/a2a/
  mod.rs, remote_transport.rs (341L), budget_cascade.rs (200L),
  message.rs (231L), dlq.rs (235L), relay.rs (148L),
  rate_limiter.rs (247L), task.rs (352L), health.rs (144L),
  telemetry.rs (138L), transfer.rs (95L), compress.rs (264L),
  cost_attribution.rs (386L), agent_card.rs (256L), a2a_compat.rs (316L)

src/core/work_graph.rs (523L) — WorkGraph, WorkNode, WorkNodeBudget, StopReason
src/core/agent_lease.rs (346L) — AgentLease, Path/Symbol leases
src/core/ocla/capsule.rs (417L) — CapsuleStore, Delta, fork/materialize
src/core/ocla/builtin/agent_gateway.rs (271L) — BuiltinAgentGateway
src/core/ocla/builtin/experiment_runner.rs (105L) — BuiltinExperimentRunner
```

## STRICT: Zero file overlap between agents!
Each agent MUST only modify the files listed in their GOAL.md.
