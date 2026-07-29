#!/usr/bin/env bash
set -euo pipefail

echo "=== Self-Deploy Verification (G10) ==="

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)"

echo "[1/3] Checking services..."
docker compose -f "$SCRIPT_DIR/docker-compose.yml" ps --format json || true

echo "[2/3] Checking gateway health..."
HEALTH="$(curl -s -o /dev/null -w '%{http_code}' http://localhost:19187/health 2>/dev/null || echo 000)"
echo "  Gateway health: HTTP $HEALTH"

echo "[3/3] Generating evidence..."
EVIDENCE_DIR="$REPO_ROOT/security/evidence"
mkdir -p "$EVIDENCE_DIR"
PASS=false
if [ "$HEALTH" = "200" ]; then
    PASS=true
fi

cat > "$EVIDENCE_DIR/g10-self-deploy-evidence.json" <<EOF
{
  "gate": "G10",
  "test": "self-deploy-verification",
  "generated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "deployment_type": "docker-compose",
  "fork_free": true,
  "base_template": "lean-ctx-deploy-template/compose/",
  "health_status": "$HEALTH",
  "pass": $PASS
}
EOF

echo
echo "Evidence written to security/evidence/g10-self-deploy-evidence.json"
if [ "$PASS" = true ]; then
    echo "PASS: Self-deploy healthy"
    exit 0
fi

echo "FAIL: Gateway health check did not return HTTP 200. Start it with: docker compose up -d"
exit 1
