#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest_path="$repo_root/rust/Cargo.toml"
generated_sbom="$repo_root/rust/sbom.json"
output_sbom="$repo_root/sbom.json"

if ! cargo deny --version >/dev/null 2>&1; then
  echo "cargo-deny is required; install it with: cargo install cargo-deny" >&2
  exit 1
fi

if ! cargo cyclonedx --version >/dev/null 2>&1; then
  echo "cargo-cyclonedx is required; install it with: cargo install cargo-cyclonedx" >&2
  exit 1
fi

cargo deny --manifest-path "$manifest_path" check
cargo cyclonedx --manifest-path "$manifest_path" --format json \
  --override-filename sbom.json
mv "$generated_sbom" "$output_sbom"
