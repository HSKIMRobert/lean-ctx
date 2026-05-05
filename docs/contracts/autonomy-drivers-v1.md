# Autonomy Drivers v1 (AutonomyDriversV1)

GitLab: `#2313`

LeanCTX enthält deterministische Helper-Driver, die Workflows schneller und zuverlässiger machen, ohne “full autonomy” zu sein. Autonomy Drivers v1 standardisiert **Trigger**, **Guards** (Budget/SLO/Boundary) und **Proof/Reports**.

## Ziele

- Deterministisch: gleiche Inputs + Policy → gleicher Driver‑Plan.
- Guarded: Budget/SLO/Boundary verhindern overfetch / token burn.
- Auditierbar: Driver‑Report speichert **welche Driver liefen** + **warum** (bounded, redaction‑safe).
- Kompatibel: Default bleibt konservativ; Opt‑In per Profile möglich.

## Driver Set (v1)

- `preload`: Session-start context bootstrap (`ctx_preload` / fallback `ctx_overview`)
- `prefetch`: bounded related reads (kleine Blast‑Radius, capped)
- `dedup`: cache dedup (`ctx_dedup`)
- `response`: response shaping (`ctx_response`) — niemals auf JSON‑Outputs

## Guards

- **Budget**: skip unter Budget-Stress (warn/throttle/block snapshots).
- **SLO**: skip wenn SLO-Throttle/Block (reduziert IO/CPU).
- **Boundary**: alle file reads durch `io_boundary::jail_and_check_path` (PathJail + secret-like checks).

## Report / Proof

- Store: `~/.lean-ctx/autonomy_drivers_v1.json` (bounded)
- Proof export: `project/.lean-ctx/proofs/autonomy-drivers-v1_<timestamp>.json` via `ctx_proof`
- Evidence: `lean-ctx proof` schreibt Artefakt, kann als Evidence Key referenziert werden (siehe Evidence Ledger).

## Relevanter Code

- Contract + Store: `rust/src/core/autonomy_drivers.rs`
- Driver hooks + execution: `rust/src/tools/autonomy.rs`
- Proof export: `rust/src/tools/ctx_proof.rs`
- Boundary: `rust/src/core/io_boundary.rs`, `rust/src/core/pathjail.rs`

