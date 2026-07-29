#!/usr/bin/env python3
"""Apply declarative GitHub and GitLab branch-protection policy."""

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
    """Raised when the policy or a remote request is invalid."""


REQUIRED_TOP_LEVEL = {
    "schema_version",
    "github",
    "gitlab",
    "scanner_source_paths",
    "scanner_source_sha256",
}


def repository_root() -> Path:
    return Path(__file__).resolve().parents[1]


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def load_policy(path: Path) -> dict:
    """Load and validate the branch-protection policy contract."""
    try:
        policy = json.loads(path.read_bytes())
    except (OSError, json.JSONDecodeError) as exc:
        raise GateError(f"cannot load policy: {exc}") from exc
    if not isinstance(policy, dict) or set(policy) != REQUIRED_TOP_LEVEL:
        raise GateError("invalid policy contract")
    if policy["schema_version"] != "leanctx.branch-protection-policy/v1":
        raise GateError("unsupported policy schema")
    github = policy["github"]
    gitlab = policy["gitlab"]
    if not all(isinstance(value, dict) for value in (github, gitlab)):
        raise GateError("invalid provider policy")
    if set(github) != {"owner", "repo", "branches"} or not all(
        isinstance(github[key], str) and github[key] for key in ("owner", "repo")
    ) or not isinstance(github["branches"], dict) or not github["branches"]:
        raise GateError("invalid GitHub policy")
    if set(gitlab) != {"host", "project", "branches"} or not all(
        isinstance(gitlab[key], str) and gitlab[key] for key in ("host", "project")
    ) or not isinstance(gitlab["branches"], dict) or not gitlab["branches"]:
        raise GateError("invalid GitLab policy")
    for branch, protection in github["branches"].items():
        required = {
            "enforce_admins", "required_status_checks",
            "required_pull_request_reviews", "required_signatures",
            "allow_force_pushes", "allow_deletions", "required_linear_history",
        }
        if not isinstance(branch, str) or not branch or set(protection) != required:
            raise GateError("invalid GitHub branch rule")
        checks = protection["required_status_checks"]
        if not isinstance(checks, dict) or set(checks) != {"strict", "contexts"}:
            raise GateError("invalid required status checks")
        if not isinstance(checks["strict"], bool) or not isinstance(checks["contexts"], list):
            raise GateError("invalid required status checks")
        if not checks["contexts"] or not all(isinstance(value, str) and value for value in checks["contexts"]):
            raise GateError("invalid required status-check context")
        if protection["required_pull_request_reviews"] is not None or not all(
            isinstance(protection[key], bool)
            for key in required - {"required_status_checks", "required_pull_request_reviews"}
        ):
            raise GateError("invalid GitHub branch protection values")
    for branch, protection in gitlab["branches"].items():
        if not isinstance(branch, str) or not branch or set(protection) != {
            "push_access_level", "merge_access_level", "allow_force_push"
        } or not isinstance(protection["push_access_level"], int) or not isinstance(
            protection["merge_access_level"], int
        ) or not isinstance(protection["allow_force_push"], bool):
            raise GateError("invalid GitLab branch rule")
    source_paths = policy["scanner_source_paths"]
    source_hashes = policy["scanner_source_sha256"]
    expected_paths = {
        "security/branch-protection-policy-v1.json",
        "scripts/apply-branch-protection.py",
        "scripts/verify-branch-protection.py",
    }
    if not isinstance(source_paths, list) or set(source_paths) != expected_paths or len(source_paths) != len(expected_paths):
        raise GateError("invalid scanner source paths")
    expected_hashes = expected_paths - {"security/branch-protection-policy-v1.json"}
    if not isinstance(source_hashes, dict) or set(source_hashes) != expected_hashes or not all(
        isinstance(digest, str) and len(digest) == 64 and set(digest) <= set("0123456789abcdef")
        for digest in source_hashes.values()
    ):
        raise GateError("invalid scanner source hashes")
    return policy


def validate_scanner_sources(root: Path, policy: dict) -> None:
    """Fail closed when a policy-enforcement script no longer matches its hash."""
    for relative, expected in policy["scanner_source_sha256"].items():
        source = root / relative
        if not source.is_file() or source.is_symlink():
            raise GateError("scanner source is missing or unsafe")
        actual = hashlib.sha256(source.read_bytes()).hexdigest()
        if actual != expected:
            raise GateError(f"scanner source digest mismatch: {relative}")


def github_request(owner: str, repo: str, branch: str, protection: dict) -> urllib.request.Request:
    """Build the GitHub branch-protection update request without credentials."""
    payload = {
        "required_status_checks": protection["required_status_checks"],
        "enforce_admins": protection["enforce_admins"],
        "required_pull_request_reviews": protection["required_pull_request_reviews"],
        "restrictions": None,
        "required_linear_history": protection["required_linear_history"],
        "allow_force_pushes": protection["allow_force_pushes"],
        "allow_deletions": protection["allow_deletions"],
        "required_conversation_resolution": False,
        "lock_branch": False,
        "allow_fork_syncing": False,
    }
    url = "https://api.github.com/repos/{}/{}/branches/{}/protection".format(
        urllib.parse.quote(owner, safe=""),
        urllib.parse.quote(repo, safe=""),
        urllib.parse.quote(branch, safe=""),
    )
    return urllib.request.Request(
        url,
        data=canonical(payload),
        method="PUT",
        headers={
            "Accept": "application/vnd.github+json",
            "Content-Type": "application/json",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )


def gitlab_request(host: str, project: str, branch: str, protection: dict) -> urllib.request.Request:
    """Build the GitLab protected-branch update request without credentials."""
    project_id = urllib.parse.quote(project, safe="")
    branch_name = urllib.parse.quote(branch, safe="")
    url = f"https://{host}/api/v4/projects/{project_id}/protected_branches/{branch_name}"
    return urllib.request.Request(
        url,
        data=canonical(protection),
        method="PUT",
        headers={"Content-Type": "application/json"},
    )


def send(request: urllib.request.Request, token: str, token_header: str) -> dict:
    """Send a JSON request without exposing an authentication value."""
    request.add_header(token_header, f"Bearer {token}" if token_header == "Authorization" else token)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            body = response.read()
            return {"status": response.status, "response": json.loads(body or b"{}")}
    except urllib.error.HTTPError as exc:
        raise GateError(f"HTTP {exc.code} for {request.get_method()} {request.full_url}") from exc
    except urllib.error.URLError as exc:
        raise GateError(f"request failed for {request.full_url}: {exc.reason}") from exc


def planned_changes(policy: dict, target: str) -> list[dict]:
    """Return a stable, credential-free description of requested updates."""
    changes = []
    if target in {"github", "all"}:
        for branch, rule in sorted(policy["github"]["branches"].items()):
            changes.append({"provider": "github", "branch": branch, "protection": rule})
    if target in {"gitlab", "all"}:
        for branch, rule in sorted(policy["gitlab"]["branches"].items()):
            changes.append({"provider": "gitlab", "branch": branch, "protection": rule})
    return changes


def apply(policy: dict, target: str, github_token: Optional[str], gitlab_token: Optional[str]) -> dict:
    """Apply selected provider rules and report HTTP result metadata."""
    changes = []
    if target in {"github", "all"}:
        if not github_token:
            raise GateError("GitHub token is required for selected target")
        github = policy["github"]
        for branch, rule in sorted(github["branches"].items()):
            result = send(github_request(github["owner"], github["repo"], branch, rule), github_token, "Authorization")
            changes.append({"provider": "github", "branch": branch, "status": result["status"]})
    if target in {"gitlab", "all"}:
        if not gitlab_token:
            raise GateError("GitLab token is required for selected target")
        gitlab = policy["gitlab"]
        for branch, rule in sorted(gitlab["branches"].items()):
            result = send(gitlab_request(gitlab["host"], gitlab["project"], branch, rule), gitlab_token, "PRIVATE-TOKEN")
            changes.append({"provider": "gitlab", "branch": branch, "status": result["status"]})
    return {"schema_version": "leanctx.branch-protection-apply-report/v1", "changed": changes}


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=repository_root())
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--github-token", default=os.environ.get("GITHUB_TOKEN"))
    parser.add_argument("--gitlab-token", default=os.environ.get("GITLAB_TOKEN"))
    subcommands = parser.add_subparsers(dest="action", required=True)
    for name in ("apply", "dry-run"):
        command = subcommands.add_parser(name)
        command.add_argument("--target", choices=("github", "gitlab", "all"), default="all")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    policy_path = args.policy or root / "security/branch-protection-policy-v1.json"
    try:
        policy = load_policy(policy_path)
        validate_scanner_sources(root, policy)
        if args.action == "dry-run":
            report = {"schema_version": "leanctx.branch-protection-dry-run/v1", "changes": planned_changes(policy, args.target)}
        else:
            report = apply(policy, args.target, args.github_token, args.gitlab_token)
    except GateError as exc:
        print(json.dumps({"error": str(exc)}, sort_keys=True), file=sys.stderr)
        return 1
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
