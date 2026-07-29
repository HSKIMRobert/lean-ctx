#!/usr/bin/env bash
set -euo pipefail

# Pilot Baseline Collection Script
# Collects 24h of baseline metrics for Shadow Pilot comparison

GATEWAY_URL="${LEANCTX_GATEWAY_URL:-http://localhost:8080}"
DURATION="${PILOT_DURATION:-86400}" # 24h default

COLLECT_END=false
if [[ "${1:-}" == "--collect-end" ]]; then
    COLLECT_END=true
    OUTPUT_DIR="${2:-./pilot-baseline}"
else
    OUTPUT_DIR="${1:-./pilot-baseline}"
fi

mkdir -p "$OUTPUT_DIR"

echo "==> Collecting baseline from $GATEWAY_URL"
echo "    Duration: ${DURATION}s"
echo "    Output: $OUTPUT_DIR"

# Health check
curl -sf "$GATEWAY_URL/health" > "$OUTPUT_DIR/health.json"
echo "    Health: OK"

# Snapshot current conformance
lean-ctx conformance --json > "$OUTPUT_DIR/conformance.json" 2>/dev/null || true

# Collect metrics snapshot
curl -sf "$GATEWAY_URL/api/admin/metrics" > "$OUTPUT_DIR/metrics-start.json" 2>/dev/null || true

# Collect current savings
lean-ctx gain --json > "$OUTPUT_DIR/savings-start.json" 2>/dev/null || true

echo "==> Baseline collection started at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "    Will complete after ${DURATION}s"
echo "    Re-run with 'pilot-baseline.sh --collect-end' to capture end snapshot"

if [[ "$COLLECT_END" == true ]]; then
    curl -sf "$GATEWAY_URL/api/admin/metrics" > "$OUTPUT_DIR/metrics-end.json" 2>/dev/null || true
    lean-ctx gain --json > "$OUTPUT_DIR/savings-end.json" 2>/dev/null || true
    lean-ctx ledger export --format settlement-evidence-v2 > "$OUTPUT_DIR/evidence.json" 2>/dev/null || true
    echo "==> End snapshot collected"
fi
