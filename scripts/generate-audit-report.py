#!/usr/bin/env python3
"""Summarize and compare bounded full-history audit reports."""

import argparse
import hashlib
import json
import pathlib
import re
import sys
from collections import Counter
from datetime import datetime, timedelta, timezone


class GateError(RuntimeError):
    """Raised when an audit input violates its reporting contract."""


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def load_json(path):
    try:
        value = json.loads(path.read_bytes())
    except (OSError, json.JSONDecodeError) as exc:
        raise GateError(f"invalid JSON input: {path}") from exc
    if not isinstance(value, dict):
        raise GateError(f"JSON object required: {path}")
    return value


def load_report(path):
    report = load_json(path)
    counts = report.get("counts")
    findings = report.get("findings", [])
    if (
        report.get("schema_version") != "leanctx.full-history-evidence/v1"
        or not isinstance(counts, dict)
        or not isinstance(counts.get("commits"), int)
        or counts["commits"] < 0
        or not isinstance(findings, list)
        or not all(isinstance(item, dict) for item in findings)
    ):
        raise GateError("invalid full-history audit report")
    # Baseline evidence predates the report-generator contract and records the
    # stable current-tree IDs without a verbose finding list.
    report.setdefault("findings", [])
    return report


def validate_schedule(schedule_path):
    schedule = load_json(schedule_path)
    paths = schedule.get("scanner_source_paths")
    hashes = schedule.get("scanner_source_sha256")
    if (
        schedule.get("schema_version") != "leanctx.audit-schedule/v1"
        or not isinstance(schedule.get("audits"), list)
        or not isinstance(schedule.get("retention"), dict)
        or not isinstance(paths, list)
        or len(paths) != len(set(paths))
        or not isinstance(hashes, dict)
        or set(paths) - {"security/audit-schedule-v1.json"} != set(hashes)
    ):
        raise GateError("invalid audit schedule policy")
    root = pathlib.Path.cwd().resolve()
    for relative, expected in hashes.items():
        candidate = pathlib.PurePosixPath(relative)
        if candidate.is_absolute() or ".." in candidate.parts or not re.fullmatch(r"[0-9a-f]{64}", expected):
            raise GateError("unsafe audit schedule scanner source")
        source = root.joinpath(*candidate.parts)
        if not source.is_file() or source.is_symlink():
            raise GateError("audit schedule scanner source missing")
        if hashlib.sha256(source.read_bytes()).hexdigest() != expected:
            raise GateError("audit schedule scanner source digest mismatch")
    return schedule


def category(finding):
    rule = str(finding.get("rule", "")).upper()
    scanner = str(finding.get("scanner", "")).lower()
    if rule.startswith("SEC") or "secret" in scanner:
        return "secret"
    if rule.startswith("IP") or "path" in scanner:
        return "forbidden path"
    return "policy violation"


def category_counts(report):
    return Counter(category(item) for item in report["findings"])


def risk_classification(report):
    counts = category_counts(report)
    if counts["secret"] or counts["policy violation"]:
        return "RED"
    if counts["forbidden path"]:
        return "YELLOW"
    return "GREEN"


def report_time(report):
    value = report.get("audit_run_at") or report.get("completed_at")
    if not isinstance(value, str):
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    return parsed.astimezone(timezone.utc) if parsed.tzinfo else None


def next_weekly_monday(value):
    """Return the next 04:00 UTC Monday after a recorded audit time."""
    candidate = value.replace(hour=4, minute=0, second=0, microsecond=0)
    candidate += timedelta(days=(0 - candidate.weekday()) % 7)
    if candidate <= value:
        candidate += timedelta(days=7)
    return candidate


def schedule_status(report, schedule):
    has_full_history = any(item.get("id") == "history-full" for item in schedule["audits"])
    last_run = report_time(report)
    if not has_full_history:
        return "NOT CONFIGURED", "not recorded", "not configured"
    if last_run is None:
        return "SCHEDULED", "not recorded", "Monday 04:00 UTC"
    return "COMPLIANT", last_run.isoformat().replace("+00:00", "Z"), next_weekly_monday(last_run).isoformat().replace("+00:00", "Z")


def markdown_summary(report, schedule):
    counts = category_counts(report)
    compliance, last_run, next_run = schedule_status(report, schedule)
    lines = [
        "# Full History Audit Summary",
        "",
        "| Metric | Value |",
        "| --- | --- |",
        f"| Total commits scanned | {report['counts']['commits']} |",
        f"| Total findings | {len(report['findings'])} |",
        f"| Forbidden path findings | {counts['forbidden path']} |",
        f"| Secret findings | {counts['secret']} |",
        f"| Policy violation findings | {counts['policy violation']} |",
        "",
        "## Audit schedule",
        "",
        f"- Last run: {last_run}",
        f"- Next scheduled: {next_run}",
        f"- Compliance status: {compliance}",
        "",
        f"## Risk assessment: {risk_classification(report)}",
    ]
    return "\n".join(lines) + "\n"


def finding_id(finding):
    value = finding.get("id")
    if isinstance(value, str) and value:
        return value
    return hashlib.sha256(canonical(finding)).hexdigest()


def baseline_finding_ids(baseline):
    ids = {finding_id(item) for item in baseline.get("findings", []) if isinstance(item, dict)}
    ids.update(value for value in baseline.get("current_tree_finding_ids", []) if isinstance(value, str))
    return ids


def compare_reports(report, baseline):
    known = baseline_finding_ids(baseline)
    findings = {finding_id(item): item for item in report["findings"]}
    new_ids = sorted(set(findings) - known)
    existing_ids = sorted(set(findings) & known)
    return {
        "schema_version": "leanctx.audit-report-delta/v1",
        "new_findings": [findings[item] for item in new_ids],
        "new_finding_ids": new_ids,
        "existing_finding_ids": existing_ids,
        "new_findings_count": len(new_ids),
    }


def history(reports_dir):
    paths = sorted(path for path in reports_dir.glob("*.json") if path.is_file())
    entries = []
    for path in paths:
        report = load_report(path)
        entries.append({
            "report": path.name,
            "commits": report["counts"]["commits"],
            "findings": len(report["findings"]),
            "risk": risk_classification(report),
        })
    return {"schema_version": "leanctx.audit-report-history/v1", "reports": entries}


def main():
    parser = argparse.ArgumentParser(description="Generate full-history audit reports.")
    sub = parser.add_subparsers(dest="action", required=True)
    summarize = sub.add_parser("summarize")
    summarize.add_argument("--input", type=pathlib.Path, required=True)
    summarize.add_argument("--schedule", type=pathlib.Path, required=True)
    check = sub.add_parser("check")
    check.add_argument("--input", type=pathlib.Path, required=True)
    check.add_argument("--baseline", type=pathlib.Path, required=True)
    trend = sub.add_parser("history")
    trend.add_argument("--reports-dir", type=pathlib.Path, required=True)
    args = parser.parse_args()
    try:
        if args.action == "summarize":
            print(markdown_summary(load_report(args.input), validate_schedule(args.schedule)), end="")
            return 0
        if args.action == "check":
            delta = compare_reports(load_report(args.input), load_report(args.baseline))
            print(canonical(delta).decode(), end="")
            return 1 if delta["new_findings_count"] else 0
        print(canonical(history(args.reports_dir)).decode(), end="")
        return 0
    except GateError as exc:
        print(f"audit report failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
