# Agent 04: Export & Verify Offline

## Preamble
Read `../e14-preamble.md` first for project context and build commands.

## Objective
Add unified ledger export and offline trace verification. Currently only
the legacy ledger has export/verify CLI commands. The unified ledger and
trace spans have no export/verify capability.

## Files to Modify
- `rust/src/core/ocla/tracing.rs` (MODIFY: add trace-to-savings join function)

## Files to Create
- `rust/src/core/ocla/ledger_export.rs` (NEW, ~200 LOC max)

## Files to Modify for module declaration
- `rust/src/core/ocla/mod.rs` (MODIFY: add `pub mod ledger_export;` alphabetically)

## Files NOT to Touch
- Do NOT modify `unified_ledger.rs` (Agents 01 and 03 handle that)
- Do NOT modify `savings_ledger/` files (Agent 02 handles that)
- Do NOT modify `builtin/savings_ledger.rs`
- Do NOT modify CLI or proxy files

## Exact Requirements

### 1. Create ledger_export.rs with export types and functions

```rust
//! Unified ledger export and offline verification.

use serde::{Deserialize, Serialize};

/// A self-contained export bundle for offline verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerExportBundle {
    pub schema_version: u32,
    pub exported_at: String,
    pub event_count: usize,
    pub hash_chain_valid: bool,
    pub total_saved_tokens: u64,
    pub events: Vec<super::unified_ledger::UnifiedSavingsEventV2>,
}

/// Result of offline verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub valid: bool,
    pub event_count: usize,
    pub chain_breaks: Vec<usize>,
    pub total_saved_tokens: u64,
    pub errors: Vec<String>,
}
```

### 2. Export function
```rust
/// Export the unified ledger as a verifiable bundle.
pub fn export_unified_ledger() -> Result<LedgerExportBundle, String> {
    let ledger = super::unified_ledger::FileUnifiedLedger::load()
        .map_err(|e| format!("failed to load unified ledger: {e}"))?;
    let events = ledger.all_events();
    let hash_valid = verify_hash_chain(&events);
    let total_saved: u64 = events.iter().map(|e| e.saved_tokens).sum();
    
    Ok(LedgerExportBundle {
        schema_version: 2,
        exported_at: chrono_or_manual_iso8601(),
        event_count: events.len(),
        hash_chain_valid: hash_valid,
        total_saved_tokens: total_saved,
        events,
    })
}
```

For the timestamp, use the same pattern as the rest of the codebase — search
for how `ts` fields are generated (likely `chrono::Utc::now()` or `SystemTime`).

### 3. Verify function
```rust
/// Verify an exported bundle offline (no ledger file needed).
pub fn verify_export_bundle(bundle: &LedgerExportBundle) -> VerificationResult {
    let mut chain_breaks = Vec::new();
    let mut errors = Vec::new();
    
    // Check hash chain
    for (i, window) in bundle.events.windows(2).enumerate() {
        if window[1].prev_hash != window[0].event_hash {
            chain_breaks.push(i + 1);
        }
    }
    
    // Check token arithmetic
    let computed_total: u64 = bundle.events.iter().map(|e| e.saved_tokens).sum();
    if computed_total != bundle.total_saved_tokens {
        errors.push(format!(
            "token total mismatch: header={}, computed={}",
            bundle.total_saved_tokens, computed_total
        ));
    }
    
    // Check event count
    if bundle.events.len() != bundle.event_count {
        errors.push(format!(
            "event count mismatch: header={}, actual={}",
            bundle.event_count, bundle.events.len()
        ));
    }
    
    VerificationResult {
        valid: chain_breaks.is_empty() && errors.is_empty(),
        event_count: bundle.events.len(),
        chain_breaks,
        total_saved_tokens: computed_total,
        errors,
    }
}
```

### 4. Hash chain verification helper
```rust
fn verify_hash_chain(events: &[super::unified_ledger::UnifiedSavingsEventV2]) -> bool {
    events.windows(2).all(|w| w[1].prev_hash == w[0].event_hash)
}
```

### 5. In tracing.rs, add trace-to-savings join helper
```rust
/// Join trace spans with savings events by trace_id.
pub fn trace_savings_summary(trace_id: &str) -> TraceSavingsSummary {
    let spans = export_trace(trace_id);
    TraceSavingsSummary {
        trace_id: trace_id.to_owned(),
        span_count: spans.len(),
        tool_names: spans.iter().filter_map(|s| s.attributes.get("tool")).cloned().collect(),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TraceSavingsSummary {
    pub trace_id: String,
    pub span_count: usize,
    pub tool_names: Vec<String>,
}
```

### 6. Tests
- `test_export_empty_ledger` — empty ledger exports valid bundle with 0 events
- `test_verify_valid_bundle` — well-formed bundle verifies as valid
- `test_verify_broken_chain` — tampered prev_hash detected
- `test_verify_token_mismatch` — wrong total_saved_tokens detected
- `test_verify_count_mismatch` — wrong event_count detected
- `test_trace_savings_summary` — summary contains span info

## Build Verification
```bash
cd rust && cargo fmt && cargo clippy --lib -- -D warnings && cargo test --lib ledger_export
```
