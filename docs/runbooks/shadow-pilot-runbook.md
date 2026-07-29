# Shadow Pilot Runbook — OBSERVE → MEASURE

## Overview

A Shadow Pilot runs lean-ctx alongside the customer's existing AI workflow
without changing their primary pipeline. It collects baseline metrics,
measures compression savings, and validates quality preservation.

## Prerequisites

- [ ] lean-ctx v3.9.14+ deployed (via Helm chart v0.4.0)
- [ ] Gateway configured with customer's LLM providers
- [ ] SLO targets agreed with customer
- [ ] SDK conformance verified (`scripts/sdk-conformance.sh`)
- [ ] Settlement evidence export tested

## Phase 1: OBSERVE (Week 1-2)

### Setup

```bash
# Deploy in shadow mode (observe only, no enforcement)
lean-ctx proxy --mode observe --log-level info

# Verify health
curl http://localhost:8080/health
lean-ctx conformance --json
```

### Baseline Collection

- Record uncompressed token usage per request.
- Measure provider latency (p50, p95, p99).
- Catalog coverage classes (languages, file types, request patterns).
- Log quality signals without enforcing.

### Daily Check

```bash
lean-ctx gain --period 24h --json  # savings summary
lean-ctx ledger export --period 24h  # evidence export
```

## Phase 2: MEASURE (Week 3-4)

### Enable Compression

```bash
# Switch to measure mode (compress + compare)
lean-ctx proxy --mode measure
```

### SLO Validation

```bash
# Run benchmark against SLO targets
lean-ctx benchmark --slo-targets default --json

# Check specific coverage classes
lean-ctx benchmark --coverage-class rust --coverage-class typescript
```

### Evidence Collection

```bash
# Export settlement evidence for the period
lean-ctx ledger export --format settlement-evidence-v2 \
  --period "2026-07-01..2026-07-31" \
  --output evidence-july.json

# Verify evidence integrity
lean-ctx ledger verify evidence-july.json
```

## Decision Points

| Metric | SLO Target | Action if Violated |
|---|---|---|
| Savings | ≥60% | Tune compression profiles |
| Quality | ≤5% degradation | Switch to safer mode |
| Latency p99 | ≤500ms | Check provider config |
| Coverage | ≥2 classes passing | Add fixtures |

## Escalation

If any SLO is violated for >24h:

1. Switch back to observe mode.
2. Collect a diagnostic bundle: `lean-ctx report-issue --include-evidence`.
3. Escalate to engineering.

## Rollback

```bash
lean-ctx proxy --mode observe  # instant rollback to shadow
lean-ctx stop                  # full stop if needed
```
