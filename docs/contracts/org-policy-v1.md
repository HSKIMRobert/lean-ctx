# Org Policy v1 (`lean-ctx.org-policy`)

GitLab: `#674` (Great Filter) · Status: **stable** (additive evolution only)

How an organisation distributes **one central, signed policy** to every endpoint
and has it enforced as an **un-bypassable floor**: a signed wrapper around a
normal [policy pack](context-policy-packs-v1.md) that each machine verifies
against a **pinned** org key before the runtime folds it in *beneath* the local
project pack. The local pack can only ever **tighten** the org floor — never
weaken it.

This is the org-grade complement to runtime enforcement (#673): #673 enforces a
*project-local* pack; #674 makes a *central* pack authoritative across a fleet.

## Roles

| Role | Does | Holds |
|---|---|---|
| **Admin** | authors a pack, `policy org sign`s it, distributes the artifact **and** the org public key | the org **private** key (in the keystore) |
| **Endpoint** | `policy org trust`s the public key once (out-of-band), `policy org install`s artifacts | the pinned **public** key |

The split is deliberate: signing proves *who* issued a policy; pinning decides
*whose* policies this machine accepts. A user cannot forge a policy (no private
key) and cannot weaken a valid one (floor merge) — that is what "un-bypassable"
means here.

## Artifact (signed JSON)

```json
{
  "schema_version": 1,
  "kind": "lean-ctx.org-policy",
  "org": "acme",
  "policy_version": "2026.06.1",
  "issued_at": "2026-06-15T00:00:00+00:00",
  "enforced": true,
  "pack_toml": "name = \"acme-floor\"\nversion = \"1.0.0\"\n…",
  "signer_public_key": "<hex 64>",
  "signature": "<hex 128>"
}
```

- `pack_toml` is the **authoritative** body — the verbatim pack source. Every
  endpoint re-parses, re-validates and re-resolves it, so a swapped body fails
  both validation *and* the signature.
- `policy_version` is the admin's rollout label (independent of the pack's own
  `version`), so `policy org status` shows which rollout a machine holds.
- `enforced = false` ⇒ **advisory**: a valid, trusted artifact distributed for
  preview (`policy org show`) but deliberately *not* folded into enforcement, so
  an admin can stage a policy before turning it on.

## Signature construction (normative)

Mirrors the [signed savings batch](../../rust/src/core/savings_ledger/signed_batch.rs)
and the [compliance report](compliance-report-v1.md):

1. Build the artifact with `signature = null` and `signer_public_key = null`.
2. `canonical = serde_json::to_vec(artifact)` with both signature fields cleared.
3. `signature = ed25519_sign(canonical)`; embed the org public key + signature as
   hex. The key is the org signing key in the
   [`agent_identity`](../../rust/src/core/agent_identity.rs) keystore, namespaced
   `org-<sanitised-name>` and selected by `--org`.

## Trust pinning

A signature only matters once the signing key is **pinned out-of-band** — the
SSH-`known_hosts` model. Two sources, checked in order:

1. `LEANCTX_ORG_TRUST_KEY` — one or more comma-separated hex public keys
   (MDM / fleet provisioning; never written by us);
2. `<config_dir>/org-trust.toml` — the set `policy org trust` maintains.

Multiple keys may be pinned (rotation). **Trust** (is this key ours?) is the
separate question from **signature validity** (were these bytes signed by that
key?). Both must hold before a policy is applied.

## Floor merge (normative)

When the org policy applies, its resolved pack is merged *beneath* the resolved
local pack so the result is never weaker than the org floor:

| Field | Merge |
|---|---|
| `deny_tools` | union (org ∪ local) |
| `allow_tools` | intersection when both set; otherwise the one set side — an allowlist can only narrow. Denied tools are excluded |
| `redaction` | union; **org wins** on a name clash (its patterns are fixed) |
| `filters.{pii,classification,injection}` | the stricter action (`off`<`warn`<`redact`<`block`) |
| `filters.blocked_labels` | union |
| `egress.forbidden_patterns` | union |
| `egress.block_secrets` | `true` wins |
| `egress.max_writes_per_min` | the smaller cap |
| `max_context_tokens` | the smaller cap |
| `audit_retention_days` | the larger window |
| `default_read_mode` | org pins it when set, else local |

The output is a normal `ResolvedPolicy` enforced exactly like a single pack
(#673), so nothing downstream needs to know a floor was applied.

## Application gate (runtime)

`runtime::active()` applies the org policy **iff** all hold, else it falls back
to the local pack alone (fail-open):

| Step | Check | On failure |
|---|---|---|
| 1 | an artifact is present (env path or installed file) | local-only |
| 2 | `verify()` — Ed25519 over canonical bytes | logged, not applied |
| 3 | signer key `is_trusted` (env or pinned) | logged, not applied |
| 4 | `enforced == true` | advisory: not applied (still previewable) |
| 5 | the pack body resolves | logged, not applied |

A tampered, untrusted or unresolvable org artifact **never bricks the agent** —
it is ignored and surfaced by `policy org status` (`signature: INVALID`,
`trust: NOT trusted`, …). This reconciles *un-bypassable* (a user cannot weaken a
*valid* org floor) with *fail-open* (a broken artifact disables the floor rather
than locking the agent out). Deployment integrity of the installed file (who may
write it) is the endpoint-management layer's responsibility, documented below.

## Pluggable source

The active artifact is located from a swappable chain, so distribution mechanism
and security checks stay independent:

1. `LEANCTX_ORG_POLICY` — an explicit path (CI, containers, MDM drop);
2. `<config_dir>/org-policy.signed.json` — where `policy org install` writes.

## CLI

```
# Admin
lean-ctx policy org key  --org <name>                       # show/create org key + pubkey
lean-ctx policy org sign <pack.toml> --org <name> \
    [--policy-version <v>] [--advisory] [-o <out.json>]

# Endpoint
lean-ctx policy org trust <pubkey-hex> [--org <name>] | --list
lean-ctx policy org untrust <pubkey-hex>
lean-ctx policy org install <artifact.json> [--trust]       # --trust = pin-on-install (TOFU)
lean-ctx policy org uninstall
lean-ctx policy org verify <artifact.json>                  # offline signature (+ trust if pinned)
lean-ctx policy org status                                  # active policy + effective floor
```

`install` re-resolves the floor immediately (`runtime::reload`) so the next agent
call enforces it. `status`/`show` print the **effective** policy (org floor ⊕
local pack) for inspection even before it is trusted/enforced, so an admin can
preview exactly what an endpoint would enforce.

## Invariants

- **Opt-in.** No pinned key ⇒ a present artifact is informational only; nothing
  is enforced. An endpoint with no trust anchor behaves exactly as before #674.
- **Un-bypassable (for valid policies).** A trusted, enforced floor cannot be
  weakened by editing `.lean-ctx/policy.toml` — every field merges toward the
  stricter side.
- **Fail-open.** Invalid/untrusted/unresolvable artifacts are ignored, never
  fatal.
- **Local-Free.** Like all enforcement, the floor constrains only the *agent*
  pipeline; it never gates a human's own local reads.

## Threat model (honest limits)

- **Endpoint write access.** Signing + pinning stop *forgery* and *silent
  weakening*. They do not, by themselves, stop a local admin who can delete the
  installed file or remove a pinned key from disabling the floor — that is an
  endpoint-management (file ownership / MDM) concern. The engine provides the
  cryptographic mechanism; fleet integrity is layered on top. `LEANCTX_ORG_*`
  env pinning + a root-owned policy path is the recommended hardened setup.
- **TOFU on `install --trust`.** Pinning the signer key at install time trusts
  whatever signed that first artifact. For high-assurance fleets, distribute the
  public key out-of-band and `trust` it explicitly before installing.
- **Key custody.** The org private key lives in the admin's keystore; its
  compromise lets an attacker mint trusted policies. Rotate by pinning a new key
  and `untrust`ing the old.

## Compatibility

Additive evolution within v1 (new optional fields). Any change to
canonicalization or signature construction requires `org-policy-v2`.

## Module map

| Piece | Path |
|---|---|
| Artifact + Ed25519 sign/verify | `rust/src/core/policy/org/model.rs` |
| Trust anchors (pin/list/remove, env) | `rust/src/core/policy/org/trust.rs` |
| Active-artifact locate/install/load | `rust/src/core/policy/org/store.rs` |
| Application gate (`active_resolved` / `status`) | `rust/src/core/policy/org/mod.rs` |
| Floor merge (stricter-wins) | `rust/src/core/policy/floor.rs` |
| Runtime fold-in | `rust/src/core/policy/runtime.rs` |
| CLI (`policy org …`) | `rust/src/cli/policy_org_cmd.rs` |
