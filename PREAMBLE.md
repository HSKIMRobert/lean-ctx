# E13 Preamble — Context Kernel Enforce Mode

## Project
lean-ctx — Rust project under `rust/`. Workspace root: this worktree's root.

## Build Commands
```bash
cd rust
cargo fmt --check
cargo clippy --lib -- -D warnings
cargo test --lib
```

## Code Style
- Zero clippy warnings (`-D warnings`)
- No `unwrap()` outside tests — use `map_err`, `ok()`, `unwrap_or_default()`
- No `#[allow(dead_code)]` unless the code is consumed in a future phase (document which)
- No mock data, no stubs, no placeholder implementations
- Doc comments on all public types and functions
- Error handling: graceful degradation (log + continue), never panic in production paths

## Key Types You Need to Know

### KernelMode (enforce.rs)
```rust
pub enum KernelMode { Shadow, Enforce, Explain }
// resolve_mode(project_root) → reads LEANCTX_KERNEL_MODE env or config.toml
```

### KernelModeConfig (activation.rs)
```rust
pub enum KernelModeConfig { Shadow, Enforce, Explain }
// Used by activation::ActivationConfig — the hot-path mode system
```

### ContextReceiptV1 (types.rs)
```rust
pub struct ContextReceiptV1 {
    pub receipt_id: String,
    pub plan_id: String,
    pub delivered_tokens: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub outcome: ReceiptOutcome,
    pub quality_signals: Vec<QualitySignal>,
    pub feedback_attribution: HashMap<String, f64>,
}
```

### ContextPlanV1 (types.rs)
```rust
pub struct ContextPlanV1 {
    pub plan_id: String,
    pub selected: Vec<PlanEntry>,
    pub budget: PlanBudget,
    pub reasoning: Option<String>,
}
```

### BoundedQueue<T> (bounded.rs)
```rust
pub struct BoundedQueue<T> { items: VecDeque<T>, max_size: usize }
// push() returns Option<T> (evicted item when full)
```

### OclaBus (ocla_bus.rs)
```rust
// Global event bus: emit(OclaEvent) → ring buffer (VecDeque, cap 1000)
// OclaEvent { event_type: String, payload: serde_json::Value, timestamp: SystemTime }
pub fn emit(event: OclaEvent)  // global
pub fn subscribe(filter: ...) -> Receiver  // global
```

### bridge.rs functions
```rust
pub fn kernel_enrich(task, project_root, budget_tokens) -> Option<KernelEnrichment>
pub fn emit_plan_event(plan: &ContextPlanV1)
pub fn emit_receipt_event(receipt: &ContextReceiptV1)
```

### enforce.rs functions
```rust
pub fn enforce_plan(plan: ContextPlanV1, policy: &ContextPolicy, mode: KernelMode) -> EnforceResult
pub fn resolve_mode(project_root: &str) -> KernelMode
```

## Import Paths
- `crate::core::context_kernel::types::{ContextPlanV1, ContextReceiptV1, ...}`
- `crate::core::context_kernel::enforce::{KernelMode, enforce_plan, resolve_mode}`
- `crate::core::context_kernel::bridge::{kernel_enrich, emit_plan_event, emit_receipt_event}`
- `crate::core::context_kernel::bounded::BoundedQueue`
- `crate::core::ocla_bus::{emit, OclaEvent}`
- `crate::core::context_kernel::activation::{ActivationConfig, KernelModeConfig}`
- `crate::core::context_kernel::policy::ContextPolicy`

## Pipeline-Breaking Patterns to Avoid
1. If you ADD a field to a struct: search ALL files for struct literal construction
2. If you ADD a tool to registry.rs: update the SSOT count test
3. If you MODIFY types.rs or wire.rs: note that OpenAPI snapshot needs regeneration
4. Run `cargo clippy --lib -- -D warnings` BEFORE committing
5. NEVER use `deny_unknown_fields` on extensible serde structs
6. Test with `cargo test --lib` — watch for SSOT tests (tool count, schema drift)
