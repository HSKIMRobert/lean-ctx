# Pilot Readiness Checklist

Verify all items before starting a Shadow Pilot. Each item links to the
relevant documentation or command.

## Infrastructure

- [ ] lean-ctx v3.9.14+ installed (`lean-ctx --version`)
- [ ] Gateway proxy running (`lean-ctx status`)
- [ ] Health check passing (`curl http://localhost:8080/health`)
- [ ] Conformance suite passing (`lean-ctx conformance --json` — 33/33)
- [ ] SDK conformance verified (`scripts/sdk-conformance.sh`)

## Configuration

- [ ] Provider credentials configured (`lean-ctx provider list`)
- [ ] Compression mode set to observe (`lean-ctx config get compression.mode`)
- [ ] Shell allowlist reviewed (`lean-ctx config get shell_allowlist`)
- [ ] Secret detection enabled (`lean-ctx config get secret_detection`)

## SLO Targets

- [ ] Savings target defined (recommend: ≥60%)
- [ ] Quality degradation ceiling defined (recommend: ≤5%)
- [ ] Latency p99 target defined (recommend: ≤500ms)
- [ ] Coverage class targets defined (recommend: ≥2 classes)

## Monitoring

- [ ] Metrics endpoint accessible (`/api/admin/metrics`)
- [ ] Ledger export working (`lean-ctx ledger export --format settlement-evidence-v2`)
- [ ] Gain reporting functional (`lean-ctx gain --json`)

## Enterprise Integration

- [ ] Settlement evidence export verified
- [ ] OCLA evidence endpoint accessible (`/api/admin/evidence/ocla`)
- [ ] Billing plan catalog accessible (`/api/billing/plans`)

## Rollback Plan

- [ ] Observe mode switch tested (`lean-ctx proxy --mode observe`)
- [ ] Full stop tested (`lean-ctx stop`)
- [ ] Diagnostic bundle tested (`lean-ctx report-issue`)
