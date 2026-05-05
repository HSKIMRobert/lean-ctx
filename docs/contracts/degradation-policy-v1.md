# Degradation Policy v1 (DegradationPolicyV1)

GitLab: `#2312`

LeanCTX tracks **budgets** and **SLOs** at runtime. Degradation Policy v1 defines a deterministic ladder for how the runtime should react when budgets/SLOs are under stress.

## Goals

- Deterministic verdict selection: same inputs + same config → same verdict.
- Traceable outputs: include stable `reason_code` + human `reason`.
- Consistent enforcement across:
  - MCP (stdio)
  - HTTP (`/v1/tools/call`)
  - Team server (HTTP + workspaces)

## Verdict ladder (v1)

1. **Warn** (default): suggest reducing scope / switching modes / tightening output density.
2. **Throttle** (optional enforcement): apply a fixed delay before tool execution.
3. **Block** (optional enforcement): stop tool execution with an explicit message.

Budget-based blocking only happens when the active role explicitly enables it (`block_at_percent < 255`).

## Config (recommendation-first)

Profile configuration:

- File: `rust/src/core/profiles.rs`
- Section: `[degradation]`
  - `enforce = true|false` (default: `false`)
  - `throttle_ms = <u64>` (default: `250`)

## Proof export

`lean-ctx proof` writes an additional artifact:

- `project/.lean-ctx/proofs/degradation-policy-v1_<timestamp>.json`

## Relevant code

- Contract + evaluation: `rust/src/core/degradation_policy.rs`
- Enforcement boundary: `rust/src/server/mod.rs` (`call_tool`)
- Proof export: `rust/src/tools/ctx_proof.rs`

