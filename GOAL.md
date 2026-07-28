# Agent 03: ContextReceiptV1 on MCP + Proxy Hot Paths

## Preamble
Read `../.worktrees/e13-preamble.md` first for project context and build commands.

## Objective
Wire `emit_receipt_event()` and `ContextReceiptV1` generation into the MCP
and proxy hot paths. Currently, receipts are created in `server/context_gate.rs`
shadow logging only. The hot paths use parallel types (McpReceipt, ChainEntry)
instead.

## Files to Modify
- `rust/src/server/post_dispatch.rs` (MODIFY: emit ContextReceiptV1 after MCP tool completion)
- `rust/src/core/context_kernel/mcp_bridge.rs` (MODIFY: generate ContextReceiptV1 from McpCallData)

## Files to Create
- NONE

## Files NOT to Touch
- Do NOT modify `types.rs` (ContextReceiptV1 already exists and is correct)
- Do NOT modify `bridge.rs` (Agent 02 is modifying that)
- Do NOT modify `ocla_bus.rs` (Agent 01 is modifying that)
- Do NOT modify `proxy/forward/mod.rs` — proxy receipt wiring is deferred
- Do NOT modify `enforce.rs` or `activation.rs`

## Exact Requirements

### 1. In mcp_bridge.rs, add a receipt generation function

Add a new public function:
```rust
/// Generate a ContextReceiptV1 from completed MCP tool call data.
pub fn generate_mcp_receipt(
    plan_id: &str,
    tool_name: &str,
    input_tokens: usize,
    output_tokens: usize,
    cache_hit: bool,
) -> super::types::ContextReceiptV1 {
    use super::types::{ContextReceiptV1, QualitySignal, ReceiptOutcome};
    use std::collections::HashMap;

    ContextReceiptV1 {
        receipt_id: format!("mcp-{}-{}", tool_name, uuid_v4_short()),
        plan_id: plan_id.to_owned(),
        delivered_tokens: output_tokens,
        cache_hits: if cache_hit { 1 } else { 0 },
        cache_misses: if cache_hit { 0 } else { 1 },
        outcome: ReceiptOutcome::Delivered,
        quality_signals: vec![],
        feedback_attribution: HashMap::new(),
    }
}
```

For `uuid_v4_short()`, use the existing pattern in the codebase — search for
how other IDs are generated (likely `format!("{:x}", rand::random::<u64>())`
or similar). If no pattern exists, use `std::time` nanoseconds as a simple unique ID.

### 2. In post_dispatch.rs, call emit_receipt_event

Find the `record_receipt_and_cost` function or the post-dispatch hook where
MCP tool results are processed. After the existing processing, add:

```rust
// Emit canonical ContextReceiptV1 on OclaBus
let receipt = crate::core::context_kernel::mcp_bridge::generate_mcp_receipt(
    "mcp-dispatch",  // plan_id (no plan was generated, use sentinel)
    &tool_name,
    input_tokens,
    output_tokens,
    cache_hit,
);
crate::core::context_kernel::bridge::emit_receipt_event(&receipt);
```

Adapt the variable names to match what's available in the function scope.
The key is: after every MCP tool call completes, a ContextReceiptV1 is
emitted on the bus.

### 3. Tests
- `test_generate_mcp_receipt_fields` — receipt has correct plan_id, tool name in receipt_id, token counts
- `test_receipt_cache_hit_counting` — cache_hit=true → cache_hits=1, cache_misses=0
- `test_receipt_cache_miss_counting` — cache_hit=false → cache_hits=0, cache_misses=1

## NOT in Scope
- Do NOT add CacheReceiptV1 (separate future work)
- Do NOT modify the proxy path (only MCP for now)
- Do NOT add new MCP tools or CLI commands
- Do NOT add new dependencies

## Build Verification
```bash
cd rust && cargo fmt && cargo clippy --lib -- -D warnings && cargo test --lib mcp_bridge
```
