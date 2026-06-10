# Context Policy Packs

Pin your team's context-governance expectations in one reviewable TOML file:
which tools agents may call, the default read mode, redaction patterns for
sensitive data, an audit-retention expectation and a context-budget cap.
Policies live in your repo, go through code review, and inherit from curated
baselines — **Policies as Code**.

```bash
lean-ctx policy list        # see what ships built in
lean-ctx policy show finance-eu
```

## Quick start

Pick the built-in closest to your posture and copy it into your repo:

```bash
mkdir -p .lean-ctx
lean-ctx policy show baseline --toml > .lean-ctx/policy.toml
lean-ctx policy validate
```

Commit `.lean-ctx/policy.toml`. From now on, governance changes are diffs.

## Built-in packs

| Pack | For |
|---|---|
| `baseline` | Any team — secret redaction (private keys, AWS, credentials, bearer tokens), 90-day audit expectation |
| `strict-redaction` | Teams handling customer data — adds JWT, GitHub/GitLab/Slack tokens, OpenAI/Anthropic/Stripe keys, DB connection strings; compact `map` reads |
| `finance-eu` | EU financial services — adds IBAN, payment cards, EU VAT, SWIFT/BIC; denies web fetches; 1-year audit expectation |
| `healthcare` | HIPAA-aligned — adds SSN, MRN, member ids, DOB, NPI; denies web fetches; 6-year audit expectation |
| `open-source` | Public repos — permissive, but secrets stay covered |

Inspect any of them resolved (`lean-ctx policy show healthcare`) or raw
(`--toml`).

## Writing your own pack

Extend a built-in and override only what differs:

```toml
name = "acme-platform"
version = "1.0.0"
description = "ACME platform team — strict redaction plus internal identifiers"
extends = "strict-redaction"

[context]
default_read_mode = "map"
deny_tools = ["ctx_url_read"]
max_context_tokens = 16000

[redaction]
employee_id = 'EMP-\d{6}'
internal_host = '\b[a-z0-9-]+\.corp\.acme\.com\b'
```

Validate before committing:

```bash
lean-ctx policy validate            # checks .lean-ctx/policy.toml
lean-ctx policy show project        # the resolved, effective policy
```

### Inheritance rules (predictable on purpose)

- **Scalars** (`default_read_mode`, `max_context_tokens`,
  `audit_retention_days`): your value wins when set.
- **`deny_tools` and `[redaction]`**: accumulate down the chain — you can add
  restrictions, never silently drop a parent's. A redaction entry with the
  same name re-points that pattern.
- **`allow_tools`**: setting it replaces the parent's list (an allowlist is a
  deliberate posture choice). A tool can never end up both allowed and denied
  — that's a validation error.

### Validation catches

- unknown/typo'd keys (`alow_tools` → hard error)
- bad names/versions, empty descriptions
- unknown read modes (must be one of the documented `ctx_read` modes)
- regexes that don't compile (with the pattern name in the error)
- `extends` to unknown packs, cycles, chains deeper than 8
- allow/deny overlaps

## Automated CGB coverage

```bash
lean-ctx policy coverage              # project pack (.lean-ctx/policy.toml)
lean-ctx policy coverage finance-eu   # any built-in or .toml path
lean-ctx policy coverage --json       # machine-readable, CI-friendly
```

`policy coverage` runs an automated **partial** assessment of a resolved
pack against the [Context Governance Benchmark](../compliance/cgb-self-assessment.md)
(v1.0-draft). It checks what a static pack analysis can honestly check —
credential redaction against synthetic fixtures (CGB-1.1), declarative rules
(1.2), regulated-identifier classes (1.3), budget cap (3.2), retention
expectation (4.3), tool posture (5.4) and egress restriction (5.5) — and
reports `PASS`/`FAIL`/`INCONCLUSIVE` per aspect.

It deliberately **never prints a maturity grade**: 7 of 32 controls are
statically touchable; the rest need the manual assessment (spec repo,
`assessment/TEMPLATE.md`). Exit code is non-zero when any check fails, so
you can gate CI on it.

## What v1 does and doesn't do

v1 is the **format, the curated baselines and the authoring tooling** — your
policy is reviewable, versioned and resolvable today. Runtime enforcement
(read-mode defaults, tool gating, budget caps and redaction applied
automatically) wires in as the follow-up; the resolved policy you see in
`policy show` is exactly what it will consume.

Full contract: `docs/contracts/context-policy-packs-v1.md`.
