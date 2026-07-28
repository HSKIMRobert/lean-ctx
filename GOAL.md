# Agent 01: OclaBus Bounded Queues + Overflow Policy

## Preamble
Read `../.worktrees/e13-preamble.md` first for project context and build commands.

## Objective
Wire the existing `BoundedQueue<T>` from `context_kernel/bounded.rs` into the
OclaBus (`core/ocla_bus.rs`) and add a configurable overflow policy.

## Files to Modify
- `rust/src/core/ocla_bus.rs` (MODIFY: replace VecDeque with BoundedQueue, add overflow policy)

## Files to Create
- NONE — all code goes into `ocla_bus.rs`

## Files NOT to Touch
- Do NOT modify any file other than `rust/src/core/ocla_bus.rs`
- Do NOT modify `bounded.rs`, `types.rs`, `bridge.rs`, or any other file

## Exact Requirements

### 1. Add OverflowPolicy enum to ocla_bus.rs
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverflowPolicy {
    DropOldest,
    DropNewest,
    Backpressure,
}
```

### 2. Add OverflowEvent struct
```rust
#[derive(Debug, Clone)]
pub struct OverflowEvent {
    pub policy: OverflowPolicy,
    pub dropped_count: usize,
    pub queue_capacity: usize,
}
```

### 3. Replace the raw VecDeque in the bus internals with BoundedQueue
The current bus uses `VecDeque` with manual `pop_front()`. Replace with:
```rust
use crate::core::context_kernel::bounded::BoundedQueue;
```
Use `BoundedQueue::push()` which returns `Option<T>` (the evicted item) when full.

### 4. Add configurable overflow policy
- Default: `DropOldest` (current behavior)
- When `DropNewest`: return the new item instead of evicting the oldest
- When `Backpressure`: log a warning and drop (no real backpressure in async-free context)
- Read policy from config: `LEANCTX_BUS_OVERFLOW` env var, fallback to `drop_oldest`

### 5. Track overflow metrics
Add counters for overflow events:
```rust
pub fn overflow_count() -> usize  // total overflows since startup
pub fn overflow_policy() -> OverflowPolicy  // currently active policy
```

### 6. Add tests
- `test_bounded_queue_replaces_vecdeque` — bus works with bounded queue
- `test_overflow_drop_oldest` — oldest item evicted when full
- `test_overflow_drop_newest` — new item rejected when full
- `test_overflow_metrics` — overflow counter increments correctly
- `test_overflow_policy_from_env` — env var configures policy

## NOT in Scope
- Do NOT modify the subscriber/filter system
- Do NOT add async/tokio — keep it sync with Mutex
- Do NOT change the public `emit()` / `subscribe()` API signatures
- Do NOT add new dependencies to Cargo.toml

## Build Verification
```bash
cd rust && cargo fmt && cargo clippy --lib -- -D warnings && cargo test --lib ocla_bus
```
