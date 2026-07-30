#!/usr/bin/env bash
# Validates hand-maintained SDK response types against the public OpenAPI paths.
#
# SDK Alignment Strategy:
# All SDK types should be generated from the OpenAPI spec.
# Until code generation is implemented, this script validates
# that SDK types match the spec.
#
# The current OpenAPI document is generated from the endpoint inventory in
# rust/src/core/openapi.rs. Each endpoint maps to its conventional response type:
# /v1/tools/call -> ToolCallResponse; /health -> HealthResponse.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC="$ROOT/rust/src/core/openapi.rs"

[[ -f "$SPEC" ]] || {
  echo "OpenAPI endpoint inventory not found: $SPEC" >&2
  exit 2
}

TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/validate-sdk-types.XXXXXX")"
trap 'rm -rf "$TMPDIR"' EXIT

endpoint_to_type() {
  local endpoint="$1"
  local segment part first rest result=""

  endpoint="${endpoint#/v1/}"
  endpoint="${endpoint#/}"
  if [[ "$endpoint" == "tools/call" ]]; then
    printf 'ToolCallResponse\n'
    return
  fi
  IFS='/' read -r -a segments <<< "$endpoint"
  for segment in "${segments[@]}"; do
    segment="${segment//./ }"
    segment="${segment//-/ }"
    segment="${segment//_/ }"
    for part in $segment; do
      first="${part%"${part#?}"}"
      rest="${part#?}"
      result+="$(printf '%s' "$first" | tr '[:lower:]' '[:upper:]')$rest"
    done
  done
  printf '%sResponse\n' "$result"
}

expected_types="$TMPDIR/expected"
while IFS= read -r endpoint; do
  endpoint_to_type "$endpoint"
done < <(sed -nE 's/^[[:space:]]*path: "([^"]+)".*/\1/p' "$SPEC") \
  | LC_ALL=C sort -u > "$expected_types"

extract_go_types() {
  sed -nE 's/^[[:space:]]*type[[:space:]]+([[:alnum:]_]+).*/\1/p' "$1"
}

extract_python_types() {
  sed -nE 's/^[[:space:]]*class[[:space:]]+([[:alnum:]_]+).*/\1/p' "$1"
}

extract_typescript_types() {
  sed -nE 's/^[[:space:]]*export[[:space:]]+(interface|type|class)[[:space:]]+([[:alnum:]_]+).*/\2/p' "$1"
}

extract_rust_types() {
  sed -nE 's/^[[:space:]]*pub[[:space:]]+(struct|enum|type)[[:space:]]+([[:alnum:]_]+).*/\2/p' "$1"
}

validate_sdk() {
  local sdk="$1"
  local type_file="$2"
  local extractor="$3"
  local actual_types="$TMPDIR/$sdk"
  local missing="$TMPDIR/$sdk.missing"
  local extra="$TMPDIR/$sdk.extra"

  if [[ ! -f "$type_file" ]]; then
    echo "ERROR: $sdk types file not found: $type_file" >&2
    return 1
  fi

  "$extractor" "$type_file" | LC_ALL=C sort -u > "$actual_types"
  comm -23 "$expected_types" "$actual_types" > "$missing"
  comm -13 "$expected_types" "$actual_types" > "$extra"

  echo "==> $sdk ($type_file)"
  if [[ -s "$missing" ]]; then
    echo "missing types:"
    sed 's/^/  - /' "$missing"
  fi
  if [[ -s "$extra" ]]; then
    echo "extra types:"
    sed 's/^/  + /' "$extra"
  fi
  if [[ ! -s "$missing" && ! -s "$extra" ]]; then
    echo "aligned"
    return 0
  fi
  return 1
}

echo "OpenAPI response types:"
sed 's/^/  - /' "$expected_types"

status=0
validate_sdk "go" "$ROOT/go-sdk/types.go" extract_go_types || status=1
validate_sdk "python" "$ROOT/py-sdk/leanctx/types.py" extract_python_types || status=1
validate_sdk "typescript" "$ROOT/ts-sdk/src/types.ts" extract_typescript_types || status=1
validate_sdk "rust" "$ROOT/clients/rust/lean-ctx-client/src/types.rs" extract_rust_types || status=1

if [[ "$status" -ne 0 ]]; then
  echo "SDK/OpenAPI type divergence detected." >&2
fi
exit "$status"
