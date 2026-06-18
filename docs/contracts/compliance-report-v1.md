# Compliance Report v1 (`lean-ctx.compliance-report`)

GitLab: `#677` (Great Filter) · Status: **stable** (additive evolution only)

The single artifact a CISO hands an auditor: OWASP-Top-10-for-Agents alignment,
framework coverage (EU AI Act / ISO 42001 / SOC 2), what enforcement **blocked
and redacted** over a date range, and the retention posture — composed into one
**Ed25519-signed**, offline-verifiable JSON document, with optional CSV and PDF
renderings for humans.

It builds on the same evidence surfaces as the [evidence bundle](evidence-bundle-v1.md):
where the bundle ships the *raw* audit segment + packs for byte-exact replay,
the compliance report ships the *aggregated, signed summary* a reader consumes
directly.

## Goals

1. **Signed & offline-verifiable**: the signature is checked from the artifact
   alone (`lean-ctx compliance verify`, or any Ed25519 verifier), with no audit
   trail and no LeanCTX install required.
2. **Honest**: enforcement counts come from the append-only audit chain; the
   chain-integrity flag and the `audit.head_hash` bind the numbers to the exact
   segment that produced them. Framework `full` claims still carry their CI test
   (inherited from `policy coverage --framework`).
3. **No fabrication**: a quiet period legitimately reports zero blocks/redactions;
   a broken local chain is reported as `chain_valid = false`, never hidden.

## Artifact (signed JSON)

```json
{
  "schema_version": 1,
  "kind": "lean-ctx.compliance-report",
  "created_at": "2026-06-15T00:00:00+00:00",
  "lean_ctx_version": "3.8.9",
  "agent_id": "local",
  "project": "<root dir name>",
  "period": { "from": "2026-05-01T00:00:00Z", "to": "2026-06-01T00:00:00Z" },
  "owasp": {
    "full": 8, "partial": 2, "minimal": 0,
    "rows": [ { "id": "OWASP-AGENT-01", "title": "Excessive Agency", "coverage": "full" } ]
  },
  "frameworks": [ /* compliance::FrameworkReport, one per framework */ ],
  "enforcement": {
    "blocked": 12,            // ToolDenied events in the period
    "redacted": 37,           // SecretDetected events in the period
    "tool_calls": 9492,       // ToolCall events (the allowed-action denominator)
    "other_security": 0,
    "by_event": [ ["tool_denied", 12], ["secret_detected", 37] ],
    "by_tool_blocked": [ ["ctx_url_read", 12] ]
  },
  "audit": {
    "entries_in_period": 9541,
    "chain_valid": true,
    "anchor_prev_hash": "genesis | <hex>",
    "head_hash": "<hex>"
  },
  "retention": {
    "policy_pack": "strict-redaction v1.0.0",
    "policy_audit_retention_days": 180,
    "plan": "business",
    "plan_source": "cached",
    "plan_audit_retention_days": 365,
    "plan_covers_policy": true
  },
  "signer_public_key": "<hex 64>",
  "signature": "<hex 128>"
}
```

## Signature construction (normative)

Mirrors the [signed savings batch](../../rust/src/core/savings_ledger/signed_batch.rs):

1. Build the report with `signature = null` and `signer_public_key = null`.
2. `canonical = serde_json::to_vec(report)` with both signature fields cleared —
   serde struct field order is stable, and the report carries no floats, so the
   bytes are reproduced identically on sign and verify.
3. `signature = ed25519_sign(canonical)`; embed the signer's public key and the
   signature as hex. The key pair is the persistent machine identity
   ([`agent_identity`](../../rust/src/core/agent_identity.rs)).

Unlike the evidence bundle (which signs a SHA-256 hex digest), the compliance
report signs the canonical bytes **directly** — the report is bounded in size,
so there is no need to hash first.

## Verification procedure (`lean-ctx compliance verify`)

| Step | Check | Failure meaning |
|---|---|---|
| 1 | JSON parses; `kind == "lean-ctx.compliance-report"` | wrong/foreign artifact |
| 2 | both `signature` and `signer_public_key` present and valid hex | unsigned or malformed |
| 3 | recompute canonical bytes (signature fields cleared); Ed25519 verifies against the embedded key | any byte changed, or signed by another key |

One flipped byte anywhere in the payload fails step 3 (covered by mutation tests
in `model.rs`). The verifier never needs the audit trail — `audit.head_hash`
inside the signed payload is what ties the report to its source segment.

## CLI

```
lean-ctx compliance report --from <rfc3339> --to <rfc3339> \
    [--framework eu-ai-act|iso42001|soc2]...   # repeatable; omit ⇒ all
    [--pack <name|path>]                        # default: project pack, else baseline
    [--format json|csv|pdf|text]                # default json
    [--out <file>]

lean-ctx compliance verify <report.json>
```

- The **signed JSON** artifact is always written (it is the verifiable
  deliverable). `--format csv|pdf` *additionally* writes that rendering beside
  it, so `--out q2.pdf --format pdf` yields `q2.pdf` + the verifiable `q2.json`.
- `--format text` prints the human report to stdout without writing a file.
- Default artifact path: `<data_dir>/compliance/report-v1_<utc-stamp>.json`.

## Exports

| Format | Signed? | Use |
|---|---|---|
| JSON | **yes** | the artifact; machine-verifiable, offline |
| CSV  | derived | flat control matrix (OWASP + framework rows) for spreadsheets |
| PDF  | derived | printable report, Helvetica, paginated — a real PDF 1.7 (no external PDF dependency; written by `core/compliance_report/pdf.rs`) |

The CSV/PDF are renderings of the signed report; their provenance is the JSON
they are emitted alongside.

## Threat model (honest limits)

* The audit chain proves **order and integrity after recording** — it cannot
  prove events were never omitted *before* being written (same limit as the
  evidence bundle).
* Counts are **relative to the local trail**; `chain_valid = false` means the
  trail's SHA-256 chain did not replay intact and the numbers should not be
  trusted until `lean-ctx audit` / the evidence bundle explains the break.
* The signer key is **self-attested** unless the auditor holds the public key
  out-of-band.
* Framework coverage rows are **statements by the engine**, reproducible by
  re-running `lean-ctx policy coverage --framework <id>` against the same pack —
  not third-party attestations.
* `retention.plan*` reflects the *commercial plan entitlement* (hosted plane);
  the **local engine is never gated** by it (Local-Free Invariant). It is shown
  so an auditor can compare the pack's declared `audit_retention_days` intent
  against the plan window that would host it.

## Compatibility

Additive evolution within v1 (new optional fields). Any change to
canonicalization or signature construction requires `compliance-report-v2`.

## Module map

| Piece | Path |
|---|---|
| Artifact + Ed25519 sign/verify/load/write | `rust/src/core/compliance_report/model.rs` |
| `build()` orchestration + pack resolution | `rust/src/core/compliance_report/mod.rs` |
| Audit date-range aggregation (blocked/redacted) | `rust/src/core/compliance_report/aggregate.rs` |
| Text + CSV rendering | `rust/src/core/compliance_report/render.rs` |
| Dependency-free PDF writer | `rust/src/core/compliance_report/pdf.rs` |
| CLI (`report` / `verify`) | `rust/src/cli/compliance_cmd.rs` |
| Framework coverage source | `rust/src/core/compliance.rs` |
| OWASP alignment source | `rust/src/core/owasp_alignment.rs` |
