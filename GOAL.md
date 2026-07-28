# Agent 02: Quality Signals in Savings Rows

## Preamble
Read `../e14-preamble.md` first for project context and build commands.

## Objective
Wire quality signals and ETPAO into savings rows at record time. Currently
`quality_signal`, `outcome`, and `efficiency_etpao` fields exist on
SavingsEvent/UnifiedSavingsEventV2 but are NEVER populated at append time.

## Files to Modify
- `rust/src/core/savings_ledger/mod.rs` (MODIFY: add quality_signal and etpao params to record functions)

## Files NOT to Touch
- Do NOT modify `unified_ledger.rs` (Agent 01 handles that)
- Do NOT modify `builtin/savings_ledger.rs` (Agent 01 handles that)
- Do NOT modify `store.rs` or `event.rs`
- Do NOT modify any CLI, proxy, or tool files

## Exact Requirements

### 1. Add quality context to record_tool_event
Find the `record_tool_event` function signature. Add optional quality parameters:
```rust
pub fn record_tool_event(
    tool: &str,
    mechanism: &str,
    original: usize,
    compressed: usize,
    model_id: &str,
    tokenizer: &str,
    agent_id: &str,
    savings: f64,
    quality_signal: Option<&str>,    // NEW
    efficiency_etpao: Option<u64>,   // NEW
) -> SavingsEvent
```

Inside the function, set the P5 fields on the `SavingsEvent`:
```rust
event.quality_signal = quality_signal.map(|s| s.to_owned());
event.efficiency_etpao = efficiency_etpao; // if field exists, or add to the event
```

### 2. Add quality context to record_read_event
Same pattern — add `quality_signal` and `efficiency_etpao` optional params.

### 3. Update ALL callers of record_tool_event
Search the file for all calls to `record_tool_event` and `record_read_event`.
Add `None, None` as the quality params to maintain backward compatibility.
IMPORTANT: only fix callers WITHIN `mod.rs` itself. If there are callers in
other files, they will get a compile error that other agents or the orchestrator
will fix by adding `None, None`.

### 4. Compute quality_signal from compression ratio
Add a helper function:
```rust
fn compression_quality_signal(original: usize, compressed: usize) -> Option<String> {
    if original == 0 {
        return None;
    }
    let ratio = 1.0 - (compressed as f64 / original as f64);
    let signal = match ratio {
        r if r >= 0.7 => "excellent",
        r if r >= 0.5 => "good",
        r if r >= 0.3 => "moderate",
        _ => "marginal",
    };
    Some(signal.to_owned())
}
```

Use this in `record_tool_event` when no explicit quality_signal is provided:
```rust
let quality = quality_signal
    .map(|s| s.to_owned())
    .or_else(|| compression_quality_signal(original, compressed));
```

### 5. Tests
- `test_quality_signal_excellent` — >=70% compression → "excellent"
- `test_quality_signal_good` — 50-69% → "good"
- `test_quality_signal_moderate` — 30-49% → "moderate"
- `test_quality_signal_marginal` — <30% → "marginal"
- `test_record_tool_event_carries_quality` — recorded event has quality_signal set

## Build Verification
```bash
cd rust && cargo fmt && cargo clippy --lib -- -D warnings && cargo test --lib savings_ledger
```
