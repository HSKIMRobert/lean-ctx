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

[filters]
pii = "redact"            # off | warn | redact | block
classification = "block"  # refuse files marked confidential/secret
injection = "redact"      # mask prompt-injection lines (OWASP LLM01)

[egress]
forbidden_patterns = ['\.prod\.acme\.internal']  # block writes/actions hitting prod
block_secrets = true      # refuse writes/actions carrying secrets or PII
max_writes_per_min = 30   # rate-limit agent writes/actions
```

Validate before committing:

```bash
lean-ctx policy validate            # checks .lean-ctx/policy.toml
lean-ctx policy show project        # the resolved, effective policy
```

### Inheritance rules (predictable on purpose)

- **Scalars** (`default_read_mode`, `max_context_tokens`,
  `audit_retention_days`): your value wins when set.
- **`deny_tools`, `[redaction]`, `filters.blocked_labels` and
  `egress.forbidden_patterns`**: accumulate down the chain — you can add
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

## How enforcement works (#673)

Once `.lean-ctx/policy.toml` exists, the resolved pack is enforced for every
agent tool call:

- **Tool gating** — a tool in `deny_tools` (or absent from an `allow_tools`
  allowlist) is refused with a `[POLICY DENIED]` message and recorded in the
  audit trail. The agent sees the refusal and moves on.
- **Redaction** — every `[redaction]` pattern (plus the built-in secret rules)
  is applied to tool output *before the model sees it*, replacing matches with
  `[REDACTED:<name>]`.
- **Default read mode** — when an agent calls `ctx_read` without a `mode`, your
  `default_read_mode` is used. An explicit `mode` always wins.
- **Token cap** — `max_context_tokens` lowers the session token budget; the
  agent hits the usual budget warning/exhausted path at your ceiling.

Guarantees that keep this safe:

- **Opt-in** — no `.lean-ctx/policy.toml`, no enforcement.
- **Never locks you out** — `ctx`, `ctx_session` and `ctx_policy` are always
  allowed, so you can inspect or switch policy even under a strict allowlist.
- **Fails open** — a pack that doesn't parse is logged and ignored rather than
  blocking work; fix it with `lean-ctx policy validate`.
- **Local-Free** — only what the *agent* does is governed. Your own reads, edits
  and `lean-ctx -c` shell commands are never gated.
- The pack is cached after first use; restart the session/daemon to pick up
  edits.

What `policy show` resolves is exactly what gets enforced.

## Input filters (#675)

The `[filters]` section adds net-new detectors that scan tool output **before it
reaches the agent** — the input side of the filter regulated teams ask for. Each
takes an action: `off`, `warn` (let through + audit), `redact` (mask matches), or
`block` (refuse the content).

```toml
[filters]
pii = "redact"                       # Swiss AHV, IBAN, payment cards, email
classification = "block"             # gate confidential/secret-marked files
injection = "block"                  # OWASP LLM01 prompt-injection
blocked_labels = ["CONFIDENTIAL", "TS//SCI"]   # optional: your own label set
```

- **PII** is checksum-validated (Luhn for cards, mod-97 for IBAN, EAN-13 for
  AHV), so a random 16-digit order number is not mistaken for a card.
- **Classification** only fires on an actual *marking* — a banner line
  (`CONFIDENTIAL` on its own line) or a `Classification:`/`Sensitivity:` field —
  not the word used in a sentence.
- **Injection** masks (or blocks) lines carrying known role-override /
  token-smuggling patterns, leaving the rest of the file intact.

Every decision is audit-logged **without leaking the data**: only the detector
class and a count are recorded (e.g. `pii:iban×2`), never the matched value. A
`block` returns a `[POLICY BLOCKED]` message in place of the content. Filters
inherit like the rest of the pack — actions override, `blocked_labels`
accumulate — and obey the same opt-in / fail-open / Local-Free guarantees.

## Egress / output DLP (#676)

Where `[filters]` scans what reaches the agent, `[egress]` scans what the agent
*writes and runs* — the output side. It checks the payload of `ctx_edit` writes
and `ctx_shell`/`ctx_execute` actions **before they execute**, so a blocked write
never touches disk and a blocked command never runs.

```toml
[egress]
forbidden_patterns = ['\.prod\.acme\.internal', 'DROP\s+TABLE']
block_secrets = true        # refuse content carrying detected secrets or PII
max_writes_per_min = 30     # sliding-window rate limit on agent writes/actions
```

- **`forbidden_patterns`** — if any regex matches the write body or command, the
  action is refused (e.g. stop the agent editing a prod connection string or
  running a destructive query).
- **`block_secrets`** — reuses your `[redaction]` patterns and the #675 PII
  detectors to stop the agent from *writing out* a secret or personal data.
- **`max_writes_per_min`** — caps how many writes/actions the agent may perform
  per minute; the next one inside the window is refused until it ages out.

A blocked egress returns a `[POLICY BLOCKED]` message and is audited
(`ToolDenied`) with a non-sensitive reason (`forbidden-pattern:…`, `secret`,
`pii:…`, `rate-limit`) — never the matched content. Egress is opt-in (no
`[egress]` section ⇒ nothing gated) and Local-Free: only the agent's tool-driven
writes/actions are checked, never your own manual edits.

Full contract: `docs/contracts/context-policy-packs-v1.md`.

## Compliance report (#677)

Policy packs *do* the governance; the compliance report *proves* it. One command
folds the engine's evidence surfaces into a single **Ed25519-signed** artifact
for a date range — the thing a CISO or auditor actually signs off on:

```bash
lean-ctx compliance report \
  --from 2026-01-01T00:00:00Z --to 2026-03-31T23:59:59Z \
  --framework eu-ai-act --framework iso42001 \
  --pack regulated-eu --format pdf --out q1-report.pdf
# → writes q1-report.json  (signed, always — the verifiable deliverable)
#   and     q1-report.pdf   (human rendering)
# Without --out, the signed JSON lands in
#   ~/.local/share/lean-ctx/compliance/report-v1_<timestamp>.json
```

The artifact bundles, for the period:

- **OWASP Top-10-for-Agents alignment** — how the active controls map to the
  agentic threat list.
- **Framework coverage** — EU AI Act / ISO 42001 / SOC2 rows, verified *live*
  against the resolved pack (not a static claim).
- **Enforcement evidence** — what was **blocked** (`ToolDenied`) and **redacted**
  (`SecretDetected`), folded from the append-only audit chain; the segment's
  `head_hash` is bound into the signed payload.
- **Retention posture** — the pack's `audit_retention_days` intent vs. your
  plan entitlement.

Honest by construction: a quiet quarter reports **zero** blocks rather than
inventing activity, and a broken local audit chain is surfaced
(`chain_valid = false`), never hidden. The signed JSON is offline-verifiable with
no audit trail and no LeanCTX install:

```bash
lean-ctx compliance verify q1-report.json
# → VALID — signature verifies (Ed25519, offline)
#     Signer key: <key-id> · Period: … · Audit head: <sha-256>
```

`--format json` (default) writes only the signed artifact; `--format csv|pdf`
additionally emits that human rendering — the PDF is a real, dependency-free
PDF 1.7. Full contract: `docs/contracts/compliance-report-v1.md`.
