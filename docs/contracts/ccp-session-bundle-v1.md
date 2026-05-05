# CCP Session Bundle v1 (CcpSessionBundleV1)

GitLab: `#2316`

CCP Session Bundles standardisieren **Export/Import** von Session-Excerpts für Replayability: Task, Findings, Decisions, Evidence (tool receipts + manual keys) sowie Governance/Policy Hashes.

## Ziele

- **Deterministisch**: gleiche Session + gleiche Policy ⇒ gleiches Bundle (bis auf `exported_at`).
- **Bounded**: feste Caps (Anzahl + Bytes).
- **Replayable**: enthält Project Identity Hash + Policy Hashes.
- **Redacted-by-default**: Exports enthalten standardmäßig keine Secret-like Inhalte.

## Bundle Format (JSON)

Top-level Felder:

- `schema_version`: `1`
- `exported_at`: RFC3339 timestamp
- `project`: project hashes (root + identity)
- `role`: `{ name, policy_md5 }`
- `profile`: `{ name, policy_md5 }`
- `session`: bounded excerpt (task/findings/decisions/files/progress/next_steps/evidence/stats)

## Privacy Levels

- `privacy="redacted"` (default): Evidence `value` wird entfernt, Strings werden durch `redaction::redact_text` gejailt.
- `privacy="full"`: nur für Admin vorgesehen; kann trotzdem redaction aktiv lassen.

## Import Semantik

- Import setzt Session-Excerpt Felder (bounded) und markiert missing/stale file paths.
- Project identity mismatch wird als Warnung reportet, Import bleibt möglich (Provenance bleibt).

## Boundedness

- Session selbst ist über MAX_* Caps bounded (`rust/src/core/session.rs`).
- Export/Import enforced zusätzlich eine max JSON byte size.

## Surface

Tool:

- `ctx_session action=export format=json|summary write=true|false path=<optional> privacy=redacted|full`
- `ctx_session action=import path=<bundle.json>`

## Relevanter Code

- Bundle contract/runtime: `rust/src/core/ccp_session_bundle.rs`
- Session store: `rust/src/core/session.rs`
- Tool surface: `rust/src/tools/ctx_session.rs` + dispatch `rust/src/server/dispatch/session_tools.rs`

