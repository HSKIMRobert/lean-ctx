#!/usr/bin/env bash
#
# tcc_sandbox.sh — #356 regression guard (macOS only).
#
# Boots the lean-ctx foreground daemon as a *TCC-standalone* process (the
# end-user LaunchAgent condition, forced via LEAN_CTX_TCC_STANDALONE=1) under a
# macOS sandbox profile that SIGKILLs the process on ANY access under
# ~/Documents. The daemon is told about a project living under ~/Documents (via
# LEAN_CTX_PROJECT_ROOT and a stored .lean-ctx.toml there) — exactly the setup
# that made #356 recur. If any boot path stats / reads / canonicalizes
# ~/Documents, the kernel kills the daemon and this test fails.
#
# This codifies the empirical method used to root-cause #356. It is the only
# check that reproduces the real end-user condition: running `lean-ctx update`
# (or tests) from a terminal masks the bug, because the terminal already holds
# the Documents TCC grant.
#
# A control run (same conditions, no sandbox) first proves the daemon boots in
# this throwaway HOME, so a sandbox-run death is unambiguously a ~/Documents
# access rather than an unrelated environment problem.
#
# Gated: only runs when LEAN_CTX_TCC_SANDBOX_TEST=1. Needs macOS + sandbox-exec.
# Binary: LEAN_CTX_BIN, else `lean-ctx` on PATH.
# Tunables: LEAN_CTX_TCC_SOAK_SECS (default 10).

set -uo pipefail

if [[ "${LEAN_CTX_TCC_SANDBOX_TEST:-0}" != "1" ]]; then
  echo "SKIP: set LEAN_CTX_TCC_SANDBOX_TEST=1 to run this regression (macOS only)"
  exit 0
fi
if [[ "$(uname)" != "Darwin" ]]; then
  echo "SKIP: macOS only (TCC is a macOS feature)"
  exit 0
fi
if ! command -v sandbox-exec >/dev/null 2>&1; then
  echo "SKIP: sandbox-exec not available"
  exit 0
fi

BIN="${LEAN_CTX_BIN:-}"
[[ -z "$BIN" ]] && BIN="$(command -v lean-ctx 2>/dev/null || true)"
if [[ -z "$BIN" || ! -x "$BIN" ]]; then
  echo "FAIL: no lean-ctx binary — set LEAN_CTX_BIN or put lean-ctx on PATH"
  exit 1
fi

SOAK="${LEAN_CTX_TCC_SOAK_SECS:-10}"
# Keep this base path SHORT: the daemon binds a Unix domain socket under
# "$HOME/Library/Application Support/lean-ctx/daemon.sock", and sun_path caps at
# ~104 bytes on macOS. TMPDIR (/var/folders/...) is already long enough to blow
# that budget, so anchor under /tmp.
ROOT_TMP="$(mktemp -d /tmp/lc.XXXXXX)"
# Resolve symlinks (/tmp -> /private/tmp, /var -> /private/var): the kernel
# matches sandbox subpath filters against the *canonical* path, and so must the
# daemon's HOME, or the deny rule silently misses.
ROOT_TMP="$(cd "$ROOT_TMP" && pwd -P)"

PIDS=()
cleanup() {
  for p in "${PIDS[@]:-}"; do
    [[ -n "$p" ]] && kill -9 "$p" 2>/dev/null || true
  done
  rm -rf "$ROOT_TMP" 2>/dev/null || true
}
trap cleanup EXIT

# Seed a throwaway HOME with a project under ~/Documents (markers + local config).
seed_home() {
  local home="$1"
  local proj="$home/Documents/proj"
  mkdir -p "$proj/.git"
  printf 'fn main() {}\n' >"$proj/main.rs"
  printf '[context]\nmax_tokens = 1234\n' >"$proj/.lean-ctx.toml"
}

# Boot the foreground daemon and report whether it survives the soak.
# Args: <home> <soak> <sandbox: 0|1>. Returns 0 if alive after soak, 1 if it died.
boot_and_soak() {
  local home="$1" soak="$2" sandboxed="$3"
  local proj="$home/Documents/proj"
  local log="$home/daemon.log"

  local -a cmd=()
  if [[ "$sandboxed" == "1" ]]; then
    local profile="$home/deny-documents.sb"
    # `(allow default)` then a later `(deny ...)` — last match wins in SBPL.
    # `file-read*` covers stat / open-read / read_dir / realpath — every TCC
    # trigger (#356). `(with send-signal SIGKILL)` turns any such access into
    # instant death of the violating process.
    cat >"$profile" <<SB
(version 1)
(allow default)
(deny file-read* (subpath "$home/Documents") (with send-signal SIGKILL))
SB
    cmd+=(sandbox-exec -f "$profile")
  fi
  cmd+=("$BIN" serve --_foreground-daemon)

  # cwd=/ mimics a real LaunchAgent; env -i isolates from the caller's XDG/LEAN_CTX.
  (
    cd / || exit 1
    exec env -i \
      HOME="$home" \
      PATH="/usr/bin:/bin:/usr/sbin:/sbin" \
      TMPDIR="${TMPDIR:-/tmp}" \
      LEAN_CTX_TCC_STANDALONE=1 \
      LEAN_CTX_PROJECT_ROOT="$proj" \
      "${cmd[@]}"
  ) >"$log" 2>&1 &
  local pid=$!
  PIDS+=("$pid")

  local s
  for ((s = 0; s < soak; s++)); do
    if ! kill -0 "$pid" 2>/dev/null; then
      return 1
    fi
    sleep 1
  done
  kill -0 "$pid" 2>/dev/null && return 0 || return 1
}

# --- Control run: prove the daemon boots in this throwaway HOME (no sandbox). ---
CTRL_HOME="$ROOT_TMP/ctrl"
seed_home "$CTRL_HOME"
if ! boot_and_soak "$CTRL_HOME" 4 0; then
  echo "FAIL(inconclusive): daemon did not stay up even WITHOUT the sandbox."
  echo "  The environment, not #356, is the problem. Daemon log:"
  echo "----- control daemon log -----"
  cat "$CTRL_HOME/daemon.log" 2>/dev/null || true
  exit 1
fi
echo "ok: control daemon booted and survived (env is sane)"

# --- Sandbox run: SIGKILL on any ~/Documents access. -------------------------
SB_HOME="$ROOT_TMP/sandbox"
seed_home "$SB_HOME"
if boot_and_soak "$SB_HOME" "$SOAK" 1; then
  echo "PASS: TCC-standalone daemon survived ${SOAK}s under deny-~/Documents sandbox"
  echo "      -> no boot path stats/reads/canonicalizes ~/Documents (#356 fixed)"
  exit 0
fi

echo "FAIL: daemon was killed under the deny-~/Documents sandbox."
echo "      A TCC-standalone boot path accessed ~/Documents — #356 has regressed."
echo "----- sandbox daemon log -----"
cat "$SB_HOME/daemon.log" 2>/dev/null || true
exit 1
