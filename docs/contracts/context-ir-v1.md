# Context IR v1 (ContextIrV1)

GitLab: `#2308`

LeanCTX produces many kinds of context (read/search/shell/provider). Context IR v1 is the **stable, versioned intermediate representation** that captures:

- **Content** (bounded excerpt)
- **Tokens** (input/output + saved)
- **Provenance** (tool + optional path/command/pattern, redaction-safe)
- **Safety** (redaction applied + boundary-mode hint)
- **Verification** (content checksum)
- **Latency / metrics** (duration + compression ratio)

## Stability & versioning

- Payload includes `schema_version` (SSOT: `rust/src/core/contracts.rs`).
- Additive changes must remain backwards-compatible for readers.

## Bounds / DoS safety

Context IR is an **observability artifact**. It is intentionally bounded:

- Max items per store: `128`
- Max excerpt per item: `4096` chars
- Max total excerpt chars across items: `65536` chars

Older items are pruned first when limits are exceeded.

## Redaction / secrets

- Stored fields are redacted using `rust/src/core/redaction.rs` and are safe to persist.
- Provenance fields (`command`, `pattern`) are redacted and bounded via excerpt limits.

## Storage & proof export

- Runtime store (local): `~/.lean-ctx/context_ir_v1.json`
- Proof export: `project/.lean-ctx/proofs/context-ir-v1_<timestamp>.json` (written by `lean-ctx proof`)

## Relevant code

- Schema + store: `rust/src/core/context_ir.rs`
- Collection points:
  - `rust/src/server/dispatch/read_tools.rs` (`ctx_read`)
  - `rust/src/server/dispatch/shell_tools.rs` (`ctx_shell`, `ctx_search`)
- Proof export: `rust/src/tools/ctx_proof.rs`

