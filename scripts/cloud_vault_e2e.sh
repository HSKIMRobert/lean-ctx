#!/usr/bin/env bash
# Live, isolated E2E for the Pro "Personal Cloud" knowledge-vault round-trip
# (GL #787). Spins up an ephemeral Postgres + the real lean-ctx-cloud-api
# (example target), drives the real engine client (seal → upload → download →
# open), and proves ciphertext-at-rest. Everything lives under a temp dir and is
# torn down on exit, so it never touches your real lean-ctx state or the prod DB.
#
#   ./scripts/cloud_vault_e2e.sh
#
# Requires a local PostgreSQL toolchain. Point at it explicitly if it is not on
# PATH or installed via Homebrew:
#
#   LEANCTX_E2E_PGBIN=/path/to/pg/bin ./scripts/cloud_vault_e2e.sh
set -uo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
RUSTDIR="$SCRIPT_DIR/../rust"

# Resolve the PostgreSQL bin dir: explicit override → pg_config → Homebrew 15.
PGBIN="${LEANCTX_E2E_PGBIN:-}"
[ -z "$PGBIN" ] && PGBIN="$(pg_config --bindir 2>/dev/null || true)"
[ -z "$PGBIN" ] && [ -d /opt/homebrew/opt/postgresql@15/bin ] && PGBIN=/opt/homebrew/opt/postgresql@15/bin
if [ -z "$PGBIN" ] || [ ! -x "$PGBIN/initdb" ]; then
  echo "PostgreSQL tools not found. Set LEANCTX_E2E_PGBIN to the bin dir."; exit 1
fi

BASE=$(mktemp -d "${TMPDIR:-/tmp}/leanctx_vault_e2e.XXXXXX")
PGDATA="$BASE/pg"
PGSOCK="$BASE/sock"
DATA="$BASE/data"
PGPORT=55439
APIPORT=18091
NEEDLE="E2E-SECRET-NEEDLE-7f3a9c21"   # keep in sync with the ignored test
DBURL="postgres://postgres@127.0.0.1:$PGPORT/leanctx"
API_PID=""

log(){ printf '\n=== %s ===\n' "$*"; }
cleanup(){
  set +e
  [ -n "$API_PID" ] && kill "$API_PID" 2>/dev/null
  "$PGBIN/pg_ctl" -D "$PGDATA" -m immediate stop >/dev/null 2>&1
  rm -rf "$BASE"
  echo "[cleanup] stopped api+postgres, removed $BASE"
}
trap cleanup EXIT

mkdir -p "$PGSOCK" "$DATA/cloud"

log "initdb"
"$PGBIN/initdb" -D "$PGDATA" -U postgres --auth=trust --encoding=UTF8 >/dev/null \
  || { echo "initdb FAILED"; exit 1; }

log "start postgres :$PGPORT"
"$PGBIN/pg_ctl" -D "$PGDATA" -l "$BASE/pg.log" \
  -o "-p $PGPORT -k $PGSOCK -c listen_addresses=127.0.0.1" -w start \
  || { echo "pg start FAILED"; cat "$BASE/pg.log" 2>/dev/null; exit 1; }

log "createdb leanctx"
"$PGBIN/createdb" -h 127.0.0.1 -p "$PGPORT" -U postgres leanctx \
  || { echo "createdb FAILED"; exit 1; }

log "build cloud-api (debug, example target)"
( cd "$RUSTDIR" && cargo build --features cloud-server --example lean-ctx-cloud-api ) \
  || { echo "build FAILED"; exit 1; }

log "start cloud-api :$APIPORT (SYNC_OPEN=1, no billing, no smtp)"
LEANCTX_CLOUD_DATABASE_URL="$DBURL" \
LEANCTX_CLOUD_SYNC_OPEN=1 \
LEANCTX_CLOUD_BIND_HOST=127.0.0.1 \
LEANCTX_CLOUD_BIND_PORT="$APIPORT" \
RUST_LOG=warn \
"$RUSTDIR/target/debug/examples/lean-ctx-cloud-api" >"$BASE/api.log" 2>&1 &
API_PID=$!

log "wait for /health"
ok=0
for _ in $(seq 1 60); do
  if curl -fsS "http://127.0.0.1:$APIPORT/health" >/dev/null 2>&1; then ok=1; break; fi
  if ! kill -0 "$API_PID" 2>/dev/null; then echo "api process died early"; cat "$BASE/api.log"; exit 1; fi
  sleep 0.3
done
[ "$ok" = 1 ] || { echo "server not healthy"; cat "$BASE/api.log"; exit 1; }

log "register account"
REG=$(curl -fsS -XPOST "http://127.0.0.1:$APIPORT/api/auth/register" \
  -H 'content-type: application/json' \
  -d '{"email":"e2e@example.com","password":"E2eTestPassw0rd!"}') \
  || { echo "register FAILED"; cat "$BASE/api.log"; exit 1; }
echo "register resp: $REG"
API_KEY=$(printf '%s' "$REG" | python3 -c 'import sys,json;print(json.load(sys.stdin)["api_key"])')
USER_ID=$(printf '%s' "$REG" | python3 -c 'import sys,json;print(json.load(sys.stdin)["user_id"])')
[ -n "${API_KEY:-}" ] || { echo "no api_key parsed"; exit 1; }
echo "user_id=$USER_ID api_key=${API_KEY:0:8}…"

log "write isolated credentials.json"
printf '{"api_key":"%s","user_id":"%s","email":"e2e@example.com","oauth_client_id":null,"oauth_client_secret":null}\n' \
  "$API_KEY" "$USER_ID" > "$DATA/cloud/credentials.json"
chmod 600 "$DATA/cloud/credentials.json"

log "run ignored E2E round-trip (real client seal/open over HTTP)"
( cd "$RUSTDIR" && LEAN_CTX_DATA_DIR="$DATA" LEAN_CTX_API_URL="http://127.0.0.1:$APIPORT" \
  cargo test --test cloud_vault_roundtrip_e2e -- --ignored --nocapture )
TEST_RC=$?

log "ciphertext-at-rest check (inspect knowledge_blobs)"
ROWS=$("$PGBIN/psql" "$DBURL" -tAc "SELECT count(*)||'|'||coalesce(sum(octet_length(blob)),0) FROM knowledge_blobs;")
echo "knowledge_blobs (rows|bytes): $ROWS"
LEAK=$("$PGBIN/psql" "$DBURL" -tAc "SELECT encode(blob,'escape') FROM knowledge_blobs;" | grep -c "$NEEDLE")
echo "plaintext-needle occurrences in stored blob: $LEAK"

log "RESULT"
if [ "$TEST_RC" = 0 ] && [ "$LEAK" = 0 ]; then
  echo "E2E RESULT: PASS — vault round-trips AND server stored only ciphertext"; RC=0
else
  echo "E2E RESULT: FAIL — test_rc=$TEST_RC leak=$LEAK"; RC=1
fi
exit $RC
