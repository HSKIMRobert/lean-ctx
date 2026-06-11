# leanctx-verify

Standalone offline verifier for LeanCTX evidence bundles
(`evidence-bundle-v1`). For auditors: **no LeanCTX installation, no
network access, no trust in the audited system required.**

```
leanctx-verify <bundle.zip> [--pubkey <hex ed25519 key>] [--json]
```

Five independent checks, each reported PASS/FAIL:

1. archive + manifest well-formed
2. every file matches its SHA-256 in the manifest (no additions/removals)
3. audit hash chain replays from anchor to head (no edit/insert/delete/reorder)
4. manifest Ed25519 signature verifies
5. per-entry signatures verify

Exit code `0` = VALID, `1` = INVALID, `2` = usage error.

Without `--pubkey` the manifest's embedded key is used (self-attested
mode — proves internal consistency only). Auditors should obtain the
organisation's public key out-of-band; see
`docs/enterprise/reading-evidence.md` for the full auditor guide.

## Design constraints

* **Independent implementation.** This crate shares no code with the
  LeanCTX engine; it implements the published contract
  (`docs/contracts/evidence-bundle-v1.md`, OCP Part 4). A PASS therefore
  attests the *specification*, not "two copies of the same code agree".
* **Minimal dependencies** (`ed25519-dalek`, `sha2`, `serde_json`, `zip`),
  release binary statically stripped.
* **Mutation-tested.** CI flips single bytes in every payload region,
  truncates the chain and swaps keys — each must produce INVALID
  (`tests/verify_bundle.rs`).

## Build

```
cargo build --release   # → target/release/leanctx-verify
```
