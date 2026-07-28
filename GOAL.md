# Agent 03: Reconciliation CI Gate

## Preamble
Read `../e14-preamble.md` first for project context and build commands.

## Objective
Add a CI-ready reconciliation test that verifies the legacy↔unified dual-write
produces zero drift. Currently `reconcile()` exists but is only tested in
unit tests, not as a CI gate.

## Files to Modify
- `rust/src/core/ocla/unified_ledger.rs` (MODIFY: add `reconcile_strict()` that returns error on any drift)

## Files NOT to Touch
- Do NOT modify `savings_ledger/mod.rs` (Agent 02 handles that)
- Do NOT modify `builtin/savings_ledger.rs` (Agent 01 handles that)
- Do NOT modify `event.rs` or `store.rs`
- Do NOT modify CLI files

## Exact Requirements

### 1. Add reconcile_strict() function
```rust
/// Strict reconciliation for CI gates: returns Err if any drift or double-booking.
pub fn reconcile_strict(&self) -> OclaResult<ReconciliationReport> {
    let report = self.reconcile()?;
    if report.token_drift != 0 || !report.double_bookings.is_empty() {
        return Err(OclaError::ValidationFailed(format!(
            "reconciliation drift: {} tokens, {} double-bookings",
            report.token_drift,
            report.double_bookings.len()
        )));
    }
    Ok(report)
}
```

### 2. Add format_reconciliation_report() for CLI output
```rust
pub fn format_reconciliation_report(report: &ReconciliationReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("Matched events: {}\n", report.matched));
    out.push_str(&format!("Unmatched legacy: {}\n", report.unmatched_legacy));
    out.push_str(&format!("Unmatched unified: {}\n", report.unmatched_unified));
    out.push_str(&format!("Token drift: {}\n", report.token_drift));
    out.push_str(&format!("Double bookings: {}\n", report.double_bookings.len()));
    if report.token_drift == 0 && report.double_bookings.is_empty() {
        out.push_str("Status: PASS\n");
    } else {
        out.push_str("Status: FAIL\n");
    }
    out
}
```

### 3. Add reconciliation coverage metric
```rust
/// Returns the percentage of legacy events that have a matching unified event.
pub fn reconciliation_coverage(&self) -> f64 {
    let report = match self.reconcile() {
        Ok(r) => r,
        Err(_) => return 0.0,
    };
    let total = report.matched + report.unmatched_legacy;
    if total == 0 {
        return 100.0;
    }
    (report.matched as f64 / total as f64) * 100.0
}
```

### 4. Tests
- `test_reconcile_strict_passes_on_clean_dual_write` — dual-write events reconcile without error
- `test_reconcile_strict_fails_on_drift` — manually inject drift → returns Err
- `test_format_reconciliation_report_pass` — format shows "PASS"
- `test_format_reconciliation_report_fail` — format shows "FAIL" with counts
- `test_reconciliation_coverage_100_on_match` — all matched → 100%
- `test_reconciliation_coverage_0_on_empty` — empty → 100% (no events = nothing to reconcile)

## Build Verification
```bash
cd rust && cargo fmt && cargo clippy --lib -- -D warnings && cargo test --lib unified_ledger
```
