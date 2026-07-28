# Agent 01: End-to-End Attribution Wiring

## Preamble
Read `../e14-preamble.md` first for project context and build commands.

## Objective
Wire `OclaRequestContext` fields (request_id, session_id, trace_id) through
all savings record paths so every `UnifiedSavingsEventV2` carries full attribution.

## Files to Modify
- `rust/src/core/ocla/unified_ledger.rs` (MODIFY: add request_id, session_id, quality_ref fields to UnifiedSavingsEventV2; extend from_savings_event)
- `rust/src/core/ocla/builtin/savings_ledger.rs` (MODIFY: propagate SavingsEvidence.context fields instead of dropping them)

## Files NOT to Touch
- Do NOT modify `savings_ledger/event.rs` or `savings_ledger/store.rs`
- Do NOT modify `savings_ledger/mod.rs`
- Do NOT modify `ocla/tracing.rs` or `ocla/types.rs`
- Do NOT modify any CLI, proxy, or tool files

## Exact Requirements

### 1. Add fields to UnifiedSavingsEventV2
Add these fields AFTER `trace_id`:
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub request_id: Option<String>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub session_id: Option<String>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub quality_ref: Option<String>,
```

### 2. Update from_savings_event() to populate new fields
The `from_savings_event` function should pass through the new fields.
Since `SavingsEvent` doesn't carry these yet, set them from the current
`OclaRequestContext` if available:
```rust
request_id: crate::core::ocla::types::OclaRequestContext::current_request_id(),
session_id: crate::core::ocla::types::OclaRequestContext::current_session_id(),
quality_ref: None, // set by Agent 02
```

If `current_request_id()` / `current_session_id()` don't exist, add simple
thread-local getters (check `current_trace_id()` pattern and replicate).

### 3. Fix BuiltinSavingsLedger to propagate context
In `record_savings()`, the `SavingsEvidence.context` fields are currently dropped.
Before calling `record_tool_event`, set the thread-local context so that
`from_savings_event` can pick it up:
```rust
fn record_savings(&self, evidence: SavingsEvidence) -> OclaResult<String> {
    // Set context for this thread so unified projection picks it up
    OclaRequestContext::set_current(evidence.context.clone());
    // ... existing record_tool_event call ...
}
```

### 4. Add query_by_trace() to FileUnifiedLedger
```rust
pub fn query_by_trace(&self, trace_id: &str) -> Vec<UnifiedSavingsEventV2> {
    self.events.iter()
        .filter(|e| e.trace_id.as_deref() == Some(trace_id))
        .cloned()
        .collect()
}
```

### 5. Tests
- `test_unified_event_carries_request_id` — after recording with context, request_id is set
- `test_unified_event_carries_session_id` — session_id propagated
- `test_query_by_trace_returns_matching` — trace query finds correct events
- `test_query_by_trace_empty_on_mismatch` — no results for unknown trace

## Build Verification
```bash
cd rust && cargo fmt && cargo clippy --lib -- -D warnings && cargo test --lib unified_ledger
```
