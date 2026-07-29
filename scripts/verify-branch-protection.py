#!/usr/bin/env python3
"""Verify GitHub branch protection against the declarative policy."""

import argparse
import hashlib
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Optional


class GateError(RuntimeError):
    """Raised when verification cannot be completed safely."""


def repository_root() -> Path:
    return Path(__file__).resolve().parents[1]


def load_policy(path: Path) -> dict:
    """Load the shared branch-protection policy without importing mutable code."""
    try:
        policy = json.loads(path.read_bytes())
    except (OSError, json.JSONDecodeError) as exc:
        raise GateError(f"cannot load policy: {exc}") from exc
    if not isinstance(policy, dict) or policy.get("schema_version") != "leanctx.branch-protection-policy/v1":
        raise GateError("invalid policy contract")
    github = policy.get("github")
    if not isinstance(github, dict) or not isinstance(github.get("owner"), str) or not isinstance(github.get("repo"), str) or not isinstance(github.get("branches"), dict):
        raise GateError("invalid GitHub policy")
    return policy


def validate_scanner_sources(root: Path, policy: dict) -> None:
    """Fail closed when either policy-enforcement script was changed in place."""
    hashes = policy.get("scanner_source_sha256")
    if not isinstance(hashes, dict) or set(hashes) != {
        "scripts/apply-branch-protection.py",
        "scripts/verify-branch-protection.py",
    }:
        raise GateError("invalid scanner source hashes")
    for relative, expected in hashes.items():
        source = root / relative
        if not source.is_file() or source.is_symlink() or not isinstance(expected, str):
            raise GateError("scanner source is missing or unsafe")
        if hashlib.sha256(source.read_bytes()).hexdigest() != expected:
            raise GateError(f"scanner source digest mismatch: {relative}")


def github_read_request(owner: str, repo: str, branch: str) -> urllib.request.Request:
    """Build the GitHub branch-protection read request without credentials."""
    url = "https://api.github.com/repos/{}/{}/branches/{}/protection".format(
        urllib.parse.quote(owner, safe=""),
        urllib.parse.quote(repo, safe=""),
        urllib.parse.quote(branch, safe=""),
    )
    return urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )


def enabled(value: object) -> object:
    """Normalize GitHub's enabled-object response representation."""
    return value.get("enabled") if isinstance(value, dict) and "enabled" in value else value


def actual_branch_protection(response: dict) -> dict:
    """Reduce GitHub's response to policy-controlled fields."""
    checks = response.get("required_status_checks")
    return {
        "enforce_admins": enabled(response.get("enforce_admins")),
        "required_status_checks": None if checks is None else {
            "strict": checks.get("strict"),
            "contexts": sorted(checks.get("contexts", [])),
        },
        "required_pull_request_reviews": response.get("required_pull_request_reviews"),
        "required_signatures": enabled(response.get("required_signatures")),
        "allow_force_pushes": enabled(response.get("allow_force_pushes")),
        "allow_deletions": enabled(response.get("allow_deletions")),
        "required_linear_history": enabled(response.get("required_linear_history")),
    }


def drift(expected: dict, response: dict) -> list[dict]:
    """Return sorted field-level differences between policy and GitHub state."""
    actual = actual_branch_protection(response)
    expected_normalized = dict(expected)
    expected_normalized["required_status_checks"] = {
        "strict": expected["required_status_checks"]["strict"],
        "contexts": sorted(expected["required_status_checks"]["contexts"]),
    }
    return [
        {"field": field, "expected": expected_normalized[field], "actual": actual[field]}
        for field in sorted(expected_normalized)
        if expected_normalized[field] != actual[field]
    ]


def fetch(request: urllib.request.Request, token: str) -> dict:
    """Fetch JSON state using a token that is never logged."""
    request.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.loads(response.read())
    except urllib.error.HTTPError as exc:
        raise GateError(f"HTTP {exc.code} for {request.full_url}") from exc
    except urllib.error.URLError as exc:
        raise GateError(f"request failed for {request.full_url}: {exc.reason}") from exc


def verify(policy: dict, github_token: str) -> dict:
    """Fetch every protected GitHub branch and produce a drift report."""
    github = policy["github"]
    findings = []
    for branch, expected in sorted(github["branches"].items()):
        response = fetch(github_read_request(github["owner"], github["repo"], branch), github_token)
        findings.extend({"branch": branch, **item} for item in drift(expected, response))
    return {"schema_version": "leanctx.branch-protection-drift/v1", "compliant": not findings, "drift": findings}


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=repository_root())
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--github-token", default=os.environ.get("GITHUB_TOKEN"))
    parser.add_argument("action", choices=("verify", "diff"))
    args = parser.parse_args(argv)
    if not args.github_token:
        print(json.dumps({"error": "GitHub token is required"}), file=sys.stderr)
        return 1
    try:
        root = args.root.resolve()
        policy = load_policy(args.policy or root / "security/branch-protection-policy-v1.json")
        validate_scanner_sources(root, policy)
        report = verify(policy, args.github_token)
    except GateError as exc:
        print(json.dumps({"error": str(exc)}, sort_keys=True), file=sys.stderr)
        return 1
    print(json.dumps(report, sort_keys=True))
    return 0 if report["compliant"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
