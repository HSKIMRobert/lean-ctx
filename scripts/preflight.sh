#!/usr/bin/env bash
#
# Local CI-parity gate — mirrors .github/workflows/ci.yml.
#
# A green run here means the *deterministic* CI jobs (Format, Clippy,
# Documentation, and the cross-platform compile) will pass. It exists because
# those failures otherwise only surface after a full ~50-min CI matrix:
# e.g. a private intra-doc link (Documentation job) or test-only code that is
# dead on Windows (Test job) — both invisible to `cargo test` / plain clippy.
#
# Usage:
#   scripts/preflight.sh [fast|full]      (default: fast)
#     fast   fmt + clippy + doc + gen_docs drift + Windows cross-compile
#     full   fast + `cargo test --lib`
#
# Bypass: not from here — use `SKIP_PREFLIGHT=1 git push` / `git push --no-verify`.
#
# CI parity (.github/workflows/ci.yml):
#   - global env: RUSTFLAGS=-Dwarnings, LEAN_CTX_EMBEDDINGS_AUTO_DOWNLOAD=0
#   - Documentation job: RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --all-features
#                        cargo run --example gen_docs --features dev-tools -- --check
#   - Clippy job:        cargo clippy --all-features -- -D warnings
#   - Format job:        cargo fmt --check
#   - Test job (Windows) compiles for x86_64-pc-windows-gnu

set -o pipefail

LEVEL="${1:-fast}"
case "$LEVEL" in
  fast|full) ;;
  *) echo "usage: $0 [fast|full]" >&2; exit 2 ;;
esac

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT/rust"

# Match CI's environment. RUSTFLAGS=-Dwarnings is applied *per step* (only where
# it must change the build fingerprint — the Windows cross-check and the full
# test build) so the fast host checks keep sharing the normal dev target cache
# instead of recompiling the whole dependency tree.
export RUSTDOCFLAGS="-Dwarnings"
export LEAN_CTX_EMBEDDINGS_AUTO_DOWNLOAD=0
# Keep proptest snappy like CI (local default is 256); override if you want more.
export PROPTEST_CASES="${PROPTEST_CASES:-64}"

# Windows test compiles the GNU target. jemalloc needs MinGW (not available on
# a plain dev box), so we cross-*check* with the default feature set minus
# jemalloc — enough to exercise the same cfg/dead-code analysis that bit us.
WIN_TARGET="x86_64-pc-windows-gnu"
WIN_FEATURES="tree-sitter,embeddings,http-server,team-server,secure-update"

BOLD="\033[1m"; CYAN="\033[1;36m"; GREEN="\033[1;32m"; RED="\033[1;31m"
YELLOW="\033[1;33m"; RESET="\033[0m"

PASSED=()
FAILED=()
SKIPPED=()

step() { # step "Label" cmd...
  local label="$1"; shift
  printf "\n${CYAN}▶ %s${RESET}\n" "$label"
  printf "${BOLD}  \$ %s${RESET}\n" "$*"
  if "$@"; then
    PASSED+=("$label")
  else
    FAILED+=("$label")
  fi
}

skip() { # skip "Label" "reason"
  printf "\n${YELLOW}⊘ %s — skipped: %s${RESET}\n" "$1" "$2"
  SKIPPED+=("$1: $2")
}

printf "${BOLD}preflight (%s) — CI-parity gate${RESET}\n" "$LEVEL"

step "Format (cargo fmt --check)" \
  cargo fmt --check

step "Clippy (--all-features -D warnings)" \
  cargo clippy --all-features -- -D warnings

step "Docs (rustdoc -D warnings)" \
  cargo doc --no-deps --all-features

step "Generated-docs drift (gen_docs --check)" \
  cargo run --quiet --example gen_docs --features dev-tools -- --check

if rustup target list --installed 2>/dev/null | grep -qx "$WIN_TARGET"; then
  step "Windows cross-compile ($WIN_TARGET)" \
    env RUSTFLAGS=-Dwarnings cargo check --target "$WIN_TARGET" --lib --tests \
      --no-default-features --features "$WIN_FEATURES"
else
  skip "Windows cross-compile ($WIN_TARGET)" \
    "target not installed — run: rustup target add $WIN_TARGET"
fi

if [ "$LEVEL" = "full" ]; then
  step "Unit tests (cargo test --lib)" \
    env RUSTFLAGS=-Dwarnings cargo test --lib --all-features
fi

# ── Summary ───────────────────────────────────────────────────────────
printf "\n${BOLD}── preflight summary ──${RESET}\n"
printf "${GREEN}  ok passed:  %d${RESET}\n" "${#PASSED[@]}"
if [ "${#SKIPPED[@]}" -gt 0 ]; then
  printf "${YELLOW}  -- skipped: %d${RESET}\n" "${#SKIPPED[@]}"
  for s in "${SKIPPED[@]}"; do printf "${YELLOW}      - %s${RESET}\n" "$s"; done
fi
if [ "${#FAILED[@]}" -gt 0 ]; then
  printf "${RED}  XX failed:  %d${RESET}\n" "${#FAILED[@]}"
  for f in "${FAILED[@]}"; do printf "${RED}      - %s${RESET}\n" "$f"; done
  printf "\n${RED}preflight FAILED — fix the above before pushing.${RESET}\n"
  exit 1
fi

printf "\n${GREEN}preflight PASSED — safe to push.${RESET}\n"
