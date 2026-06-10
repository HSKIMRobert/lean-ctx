# Context Policy Packs v1 (GL #489)

Declarative, versioned governance presets — "Context-Policies as Code". A pack
pins a team's context-governance expectations in reviewable TOML: default read
mode, allowed/denied tools, redaction patterns, an audit-retention expectation
and a context-budget cap. The reduced, solo-viable slice of #377/#403/#404.

v1 ships the **format, validation, resolution, five curated built-ins and the
`lean-ctx policy` CLI**. Runtime enforcement, pack signing and registry
distribution are explicit follow-ups (see *Out of scope*).

## Format

A pack is one TOML file. The project pack lives at `.lean-ctx/policy.toml`.

```toml
name = "acme-internal"          # lowercase letters, digits, hyphens
version = "1.0.0"               # MAJOR.MINOR.PATCH (digits only)
description = "ACME engineering baseline"
extends = "strict-redaction"    # optional: single inheritance, built-in parent

[context]                       # all fields optional
default_read_mode = "map"       # auto|full|map|signatures|diff|task|reference|aggressive|entropy
allow_tools = ["ctx_read", "ctx_search"]   # when set: only these
deny_tools = ["ctx_url_read"]   # always additive down the chain
max_context_tokens = 12000      # > 0
audit_retention_days = 365      # governance intent (hosted plane enforces its plan window)

[redaction]                     # name -> regex, matched before content enters context
employee_id = 'EMP-\d{6}'
```

Unknown keys are **rejected** (`deny_unknown_fields`) so a typo like
`alow_tools` fails validation instead of silently weakening a policy.

## Inheritance (`extends`)

Single inheritance against the built-in registry, max depth 8, cycles
rejected. Semantics are security-first and predictable:

| Field | Rule |
|---|---|
| `default_read_mode`, `max_context_tokens`, `audit_retention_days` | child **overrides** when set |
| `deny_tools` | **accumulates** (parent restrictions can never be dropped) |
| `[redaction]` | **accumulates**; a child entry with the same name re-points that pattern |
| `allow_tools` | child **overrides** when set (an allowlist is a posture choice, not a set union) |

After folding, a resolved `allow_tools` colliding with an accumulated deny is
an error (`AllowDenyOverlap`) — a pack cannot both allow and deny a tool.

## Built-in packs

| Pack | Extends | Posture |
|---|---|---|
| `baseline` | — | secret redaction (PEM keys, AWS, credential assignments, bearer tokens), `auto` mode, 90-day audit expectation |
| `strict-redaction` | baseline | + JWT/GitHub/GitLab/Slack/OpenAI/Anthropic/Stripe/DB-URL coverage, `map` mode, 180 days |
| `finance-eu` | strict-redaction | + IBAN/payment-card/EU-VAT/SWIFT, denies `ctx_url_read`, 12 k token cap, 365 days |
| `healthcare` | strict-redaction | + SSN/MRN/member-id/DOB/NPI (HIPAA-aligned), denies `ctx_url_read`, 12 k cap, 2 190 days |
| `open-source` | baseline | permissive, keeps secret coverage, 30 days |

Built-ins are embedded at compile time (`include_str!`) and covered by tests:
every pack must parse, validate, resolve and retain the baseline secret
coverage; the regulated packs must deny web fetches and pin budgets.

## CLI

```
lean-ctx policy list                  # built-ins + project pack (if any)
lean-ctx policy show <name> [--toml]  # resolved effective policy / raw TOML
lean-ctx policy show project          # the .lean-ctx/policy.toml pack
lean-ctx policy show ./custom.toml    # any pack file
lean-ctx policy validate [path]       # lint (default .lean-ctx/policy.toml); exit 1 on INVALID
```

`show --toml` prints the **unresolved** pack definition — the natural starting
point for an org-specific pack:

```
lean-ctx policy show baseline --toml > .lean-ctx/policy.toml
```

## Error vocabulary

`PolicyError` names the offending field and value; the CLI prints it verbatim:
`Toml`, `InvalidName`, `InvalidVersion`, `EmptyDescription`,
`UnknownReadMode`, `BadRegex{pattern_name}`, `ZeroMaxTokens`,
`AllowDenyOverlap`, `UnknownParent`, `ExtendsCycle`, `ExtendsTooDeep`.

## Out of scope (follow-ups)

1. **Runtime enforcement** — applying read-mode/tool-gating/budget/redaction
   at the hot path. Deliberately decoupled so v1 carries zero hot-path churn
   (lands after the in-flight engine refactor merges).
2. **Signing + trust pipeline**, registry/marketplace distribution (#403/MKT).
3. **Conformance scoring** against live project telemetry (#426 benchmark).
4. Multi-file packs, non-built-in parents (`extends` against local files).

## Module map

| Piece | Path |
|---|---|
| Types, parse, validate, resolve | `rust/src/core/policy/mod.rs` |
| Built-in registry | `rust/src/core/policy/builtin.rs` |
| Built-in pack sources | `rust/src/core/policy/builtin/*.toml` |
| CLI | `rust/src/cli/policy_cmd.rs` (dispatch key `policy`) |
| Authoring guide | `docs/guides/policy-packs.md` |
