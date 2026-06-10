#!/usr/bin/env python3
"""SDK release-coupling check (GL #395).

Runs on every engine release (and in CI): verifies that the three first-party
SDKs are release-compatible with the engine being shipped.

Hard failures (exit 1):
  * an SDK does not list the engine's current `http_mcp` contract version in
    its SUPPORTED_HTTP_CONTRACT_VERSIONS — releasing the engine would break it.

Warnings (exit 0, GitHub annotations):
  * SDK versions have drifted >1 minor from each other — releases should keep
    the SDK family moving together.

Sources of truth (no duplication):
  * engine contract version: `rust/src/core/contracts.rs` (http-mcp entry)
  * SDK supported versions:  conformance kits in each SDK
  * SDK package versions:    pyproject.toml / package.json / Cargo.toml
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

FAILURES: list[str] = []
WARNINGS: list[str] = []


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def engine_http_contract_version() -> int:
    """The engine's current http_mcp contract version from the contract registry."""
    src = read("rust/src/core/contracts.rs")
    m = re.search(r'doc\(\s*"http-mcp",\s*"[^"]+",\s*(\d+)', src)
    if not m:
        sys.exit("FATAL: http-mcp entry not found in rust/src/core/contracts.rs")
    return int(m.group(1))


def sdk_supported_versions() -> dict[str, list[int]]:
    """SUPPORTED_HTTP_CONTRACT_VERSIONS as declared by each SDK conformance kit."""
    sources = {
        "python": ("clients/python/leanctx/conformance.py",
                   r"SUPPORTED_HTTP_CONTRACT_VERSIONS\s*[:=].*?\(([^)]*)\)"),
        "typescript": ("cookbook/sdk/src/conformance.ts",
                       r"SUPPORTED_HTTP_CONTRACT_VERSIONS[^=\n]*=\s*\[([^\]]*)\]"),
        "rust": ("clients/rust/lean-ctx-client/src/conformance.rs",
                 r"SUPPORTED_HTTP_CONTRACT_VERSIONS:\s*&\[u\d+\]\s*=\s*&\[([^\]]*)\]"),
    }
    out: dict[str, list[int]] = {}
    for sdk, (path, pattern) in sources.items():
        m = re.search(pattern, read(path))
        if not m:
            sys.exit(f"FATAL: SUPPORTED_HTTP_CONTRACT_VERSIONS not found in {path}")
        out[sdk] = [int(v) for v in re.findall(r"\d+", m.group(1))]
    return out


def sdk_package_versions() -> dict[str, tuple[int, int, int]]:
    def semver(raw: str, origin: str) -> tuple[int, int, int]:
        m = re.match(r"(\d+)\.(\d+)\.(\d+)", raw)
        if not m:
            sys.exit(f"FATAL: unparsable version {raw!r} in {origin}")
        return (int(m.group(1)), int(m.group(2)), int(m.group(3)))

    py = re.search(r'^version\s*=\s*"([^"]+)"', read("clients/python/pyproject.toml"), re.M)
    ts = json.loads(read("cookbook/sdk/package.json"))["version"]
    rs = re.search(
        r'^version\s*=\s*"([^"]+)"',
        read("clients/rust/lean-ctx-client/Cargo.toml"),
        re.M,
    )
    if not py or not rs:
        sys.exit("FATAL: SDK package version not found")
    return {
        "python": semver(py.group(1), "pyproject.toml"),
        "typescript": semver(ts, "package.json"),
        "rust": semver(rs.group(1), "Cargo.toml"),
    }


def main() -> int:
    engine_contract = engine_http_contract_version()
    supported = sdk_supported_versions()
    versions = sdk_package_versions()

    print(f"engine http_mcp contract: v{engine_contract}")
    for sdk in sorted(supported):
        v = ".".join(map(str, versions[sdk]))
        print(f"  {sdk:<11} {v:<8} supports http_mcp {supported[sdk]}")

    # Gate 1 (hard): every SDK must support the engine's contract version.
    for sdk, vers in supported.items():
        if engine_contract not in vers:
            FAILURES.append(
                f"{sdk} SDK supports http_mcp {vers} but the engine ships "
                f"v{engine_contract} — update the SDK before releasing"
            )

    # Gate 2 (soft): SDK family should not drift apart by >1 minor.
    majors = {v[0] for v in versions.values()}
    if len(majors) > 1:
        WARNINGS.append(f"SDK major versions diverge: { {k: v[0] for k, v in versions.items()} }")
    else:
        minors = {sdk: v[1] for sdk, v in versions.items()}
        if max(minors.values()) - min(minors.values()) > 1:
            WARNINGS.append(
                f"SDK minor versions drift >1: {minors} — plan a catch-up release"
            )

    for w in WARNINGS:
        print(f"::warning title=SDK version drift::{w}")
    for f in FAILURES:
        print(f"::error title=SDK release gate::{f}")

    if FAILURES:
        return 1
    print("OK: all SDKs are release-compatible with the engine")
    return 0


if __name__ == "__main__":
    sys.exit(main())
