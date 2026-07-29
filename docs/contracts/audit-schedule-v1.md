# Audit Schedule Contract v1

## Purpose

This contract defines automated security-audit cadence, evidence retention, and
the response expected for findings produced by those audits.

## Audit catalogue

| Audit | Frequency | Check | Evidence |
| --- | --- | --- | --- |
| History delta | Every push | New forbidden paths and secret-pattern introductions since the approved baseline | CI log |
| Full history | Monday 04:00 UTC | Every reachable commit for forbidden paths and secret patterns | `full-history-audit` artifact |
| Secret rotation | Monthly | Credentials approaching or exceeding their allowed lifetime | CI log and report |
| Dependency audit | Every push | Cargo dependency advisories | CI log |
| CodeQL | Push and weekly | Code and workflow security analysis | Code scanning alert |

The authoritative machine-readable catalogue is
`security/audit-schedule-v1.json`. Its source digests bind the report generator
and the scheduled workflow to that catalogue.

## Full-history audit

The scheduled workflow checks out all reachable history, runs
`history-policy-gate.py full-audit`, uploads its JSON report for 90 days, and
compares findings to the approved full-history baseline. It runs read-only and
does not receive persisted checkout credentials.

`generate-audit-report.py summarize` emits a Markdown CI summary. Its risk
classification is GREEN when no findings exist, YELLOW for forbidden-path-only
findings, and RED for secret or other policy-violation findings. A missing run
timestamp is reported as `SCHEDULED`; it is not fabricated from wall-clock
time.

## Escalation and SLA

Any new finding fails the comparison step. The workflow owner opens or links a
security incident, preserves the artifact, and assigns an accountable triager.
All findings must be triaged within 72 hours. Secret findings are RED: revoke
or rotate the exposed credential immediately, assess repository history, and
record the remediation decision. Forbidden-path findings are YELLOW until the
content is removed, approved, or explicitly baselined.

## Baseline updates

Only a security reviewer may update the baseline after triage. The reviewer
must run a complete audit from a clean checkout, verify the policy and scanner
source digests, document the accepted findings and rationale, then commit the
canonical evidence and its policy digest together. Baselines never suppress
unreviewed findings; the comparison is by stable finding identity.

## Retention

Full-history audit artifacts are retained for 90 days. Approved baseline
evidence is retained permanently. Incident records and the rationale for each
baseline change follow the project security-record retention policy.
