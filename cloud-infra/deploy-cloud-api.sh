#!/usr/bin/env bash
# deploy-cloud-api.sh — build + (re)deploy the lean-ctx cloud API container.
#
# Idempotent, with an image backup and a health-gated rollback. Run this ON the
# Docker host (pounce-server). The build context is the repo root that contains
# `rust/` and `cloud-infra/` (the Dockerfile does `COPY rust ./rust`).
#
# Runtime config + secrets are inherited from the currently-running container
# (LEANCTX_* / DATABASE_URL env) so this script never has to embed credentials.
# On the very first deploy (no running container) pass an env file via $ENV_FILE.
#
# Usage:
#   ./cloud-infra/deploy-cloud-api.sh
#   ENV_FILE=/path/to/cloud-api.env ./cloud-infra/deploy-cloud-api.sh   # bootstrap
set -euo pipefail

NAME="lean-ctx-cloud-api"
IMAGE="lean-ctx-cloud-api:latest"
BACKUP_IMAGE="lean-ctx-cloud-api:backup"
NETWORK="coolify"
PORT="8088"
DOCKERFILE="cloud-infra/Dockerfile.cloud-api"
CURL_IMAGE="curlimages/curl:latest"

# Resolve to the repo root (this script lives in cloud-infra/).
cd "$(dirname "$0")/.."

log() { printf '\033[36m==>\033[0m %s\n' "$*"; }
die() { printf '\033[31mERROR:\033[0m %s\n' "$*" >&2; exit 1; }

[ -f "$DOCKERFILE" ] || die "missing $DOCKERFILE (run from a checkout with rust/ + cloud-infra/)"

# ── 1. Capture the live runtime env (secrets stay on the host) ────────────────
ENV_FILE_TMP="$(mktemp)"
trap 'rm -f "$ENV_FILE_TMP"' EXIT
if [ -n "${ENV_FILE:-}" ]; then
  [ -f "$ENV_FILE" ] || die "ENV_FILE=$ENV_FILE not found"
  grep -E '^(LEANCTX_|DATABASE_URL=)' "$ENV_FILE" > "$ENV_FILE_TMP" || true
elif docker inspect "$NAME" >/dev/null 2>&1; then
  docker inspect "$NAME" --format '{{range .Config.Env}}{{println .}}{{end}}' \
    | grep -E '^(LEANCTX_|DATABASE_URL=)' > "$ENV_FILE_TMP" || true
fi
ENV_COUNT="$(wc -l < "$ENV_FILE_TMP" | tr -d ' ')"
[ "$ENV_COUNT" -ge 1 ] || die "no LEANCTX_* env captured — pass ENV_FILE=… for the first deploy"
log "captured $ENV_COUNT runtime env vars (LEANCTX_* / DATABASE_URL)"

# ── 2. Build the new image ────────────────────────────────────────────────────
log "building $IMAGE:new from $DOCKERFILE"
docker build -f "$DOCKERFILE" -t "${IMAGE%:*}:new" .

# ── 3. Back up the currently-deployed image, then promote the new one ─────────
if docker image inspect "$IMAGE" >/dev/null 2>&1; then
  docker tag "$IMAGE" "$BACKUP_IMAGE"
  log "backed up current image -> $BACKUP_IMAGE"
fi
docker tag "${IMAGE%:*}:new" "$IMAGE"
docker rmi "${IMAGE%:*}:new" >/dev/null 2>&1 || true

# ── 4. Swap the container ─────────────────────────────────────────────────────
log "replacing container $NAME"
docker rm -f "$NAME" >/dev/null 2>&1 || true
docker run -d \
  --name "$NAME" \
  --network "$NETWORK" \
  --restart unless-stopped \
  --env-file "$ENV_FILE_TMP" \
  "$IMAGE" >/dev/null

# ── 5. Health-gate (rollback on failure) ──────────────────────────────────────
# Reach the new container by name on the shared docker network via a throwaway
# curl container — the cloud API port is not host-published (Traefik fronts it).
probe() {
  docker run --rm --network "$NETWORK" "$CURL_IMAGE" \
    -fsS --max-time 4 "http://${NAME}:${PORT}$1" 2>/dev/null
}

log "waiting for /health …"
healthy=0
for _ in $(seq 1 30); do
  if probe /health >/dev/null; then healthy=1; break; fi
  sleep 2
done

if [ "$healthy" -eq 1 ] && probe /api/leaderboard >/dev/null; then
  log "healthy: /health + /api/leaderboard OK"
  docker rmi "$BACKUP_IMAGE" >/dev/null 2>&1 || true
  log "DONE — $NAME is live on $IMAGE"
else
  printf '\033[31m==> health check FAILED — rolling back\033[0m\n' >&2
  docker logs --tail 40 "$NAME" 2>&1 | sed 's/^/    /' >&2 || true
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  if docker image inspect "$BACKUP_IMAGE" >/dev/null 2>&1; then
    docker tag "$BACKUP_IMAGE" "$IMAGE"
    docker run -d --name "$NAME" --network "$NETWORK" --restart unless-stopped \
      --env-file "$ENV_FILE_TMP" "$IMAGE" >/dev/null
    log "rolled back to previous image ($BACKUP_IMAGE)"
  fi
  die "deploy aborted; previous version restored"
fi
