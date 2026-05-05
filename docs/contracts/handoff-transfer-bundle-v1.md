# Handoff Transfer Bundle v1 (HandoffTransferBundleV1)

GitLab: `#2320` (Subtickets: `#2347–#2351`)

Das **Handoff Transfer Bundle** ist ein portables, versioniertes Export/Import‑Format für Handoffs über Tools/Teams/Transportflächen hinweg.
Es ergänzt `HandoffLedgerV1` um **projektgebundene Identität**, **boundedness** und **privacy‑aware Export** (redacted‑by‑default), plus eine **Artefakt‑Übersicht** für Replayability.

## Ziele

- **Deterministisch**: gleicher Zustand ⇒ gleiches Bundle (bis auf `exported_at`).
- **Bounded**: klare Caps (Bytes/Counts) verhindern „unendlich große“ Handoffs.
- **Redacted-by-default**: Export ist standardmäßig privacy-safe; `full` nur für `admin`.
- **Portable**: Bundle ist eine einzelne JSON Datei; Import ist **additiv** (kein Overwrite von lokalen Indizes/Caches).
- **Replayability**: enthält project identity hashes + Artefakt‑Übersicht (Proofs).

## Format (JSON)

Top-level Felder:

- `schema_version`: `1`
- `exported_at`: RFC3339 timestamp
- `privacy`: `"redacted" | "full"`
- `project`:
  - `project_root_hash`: best-effort hash des Project Roots
  - `project_identity_hash`: best-effort hash der Project Identity
- `ledger`: `HandoffLedgerV1` (ggf. redacted)
- `artifacts`:
  - `resolved`: Liste konfigurierte Context‑Artifacts (`.leanctxcontextartifacts.json` etc.)
  - `proof_files`: Liste `.lean-ctx/proofs/*` (basename + md5 + bytes), bounded
  - `warnings`: warnings beim artifact resolve/listing

## Privacy Levels

- `privacy="redacted"` (default):
  - `ledger.project_root` wird entfernt
  - `ledger.session_snapshot` wird entfernt
  - freie Textfelder (Task/Findings/Decisions/Next steps, Knowledge values, curated ref contents) werden über `redaction::redact_text` gejailt
- `privacy="full"`:
  - nur erlaubt für Role `admin`
  - wenn Redaction für `admin` aktiv ist, bleibt Redaction konsistent aktiv

## Import Semantik

- Import validiert `schema_version` (hart).
- Project identity mismatch wird als **WARN** reportet (Import bleibt möglich).
- Import ist **additiv**:
  - optional: workflow anwenden
  - optional: session excerpt anwenden
  - optional: knowledge facts importieren (policy‑aware)
- **Non-goal**: kein Overwrite bestehender lokaler Indizes/Caches.

## Surface

Tool:

- `ctx_handoff action=export format=json|summary write=true|false path=<optional> privacy=redacted|full`
- `ctx_handoff action=import path=<bundle.json> apply_workflow=true|false apply_session=true|false apply_knowledge=true|false`

## Relevanter Code

- Bundle contract/runtime: `rust/src/core/handoff_transfer_bundle.rs`
- Ledger: `rust/src/core/handoff_ledger.rs`
- Tool surface: `rust/src/server/dispatch/session_tools.rs` + `rust/src/tool_defs/granular.rs` + `rust/src/tools/ctx_handoff.rs`

