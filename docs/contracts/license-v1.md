# License v1 (`lean-ctx.license`)

GitLab: `#667` (Outcome-Based Pricing) · Status: **stable** (additive evolution only)

How a **self-hosted / air-gapped** deployment unlocks Enterprise entitlements
**offline** — no control-plane round-trip. A [`LicenseV1`](../../rust/src/core/license/model.rs)
is a vendor-signed JSON file that elevates the [effective plan](billing-plane-v1.md)
to the plan it grants (typically `enterprise`). It is the product enabler for
outcome-fee pilots with data-residency customers (banks, UN agencies) that cannot
reach the hosted billing plane.

This is the offline complement to the cloud plan resolver: the hosted path
([`cloud_client::refresh_effective_plan`](../../rust/src/cloud_client.rs)) confirms
a plan against the backend; a license grants one with a cryptographic proof and
**zero network**.

## Local-Free Invariant

A license **only ever unlocks commercial/hosted entitlements**. Neither its
absence nor its expiry can disable any local capability — the local engine has no
entitlement checks at all. The conformance test
[`tests/local_free_invariant.rs`](../../rust/tests/local_free_invariant.rs)
(`local_features_are_unaffected_by_license_or_plan_env`) guards this: setting
`LEAN_CTX_LICENSE` to any value must not change a single local feature.

## Roles

| Role | Does | Holds |
|---|---|---|
| **Vendor** | `license keygen` (bootstrap), `license issue` (sign a license for a customer) | the vendor **private** key |
| **Customer** | `license install` / `status` / `verify` on the self-host | the installed artifact; trusts the vendor **public** key (compiled-in anchor) |

Signing proves *who* issued the license; the **compiled-in trust anchor** decides
*whose* licenses a machine accepts. A customer cannot mint their own Enterprise
plan (no private key, and an arbitrary key is not a trusted anchor).

## Artifact (signed JSON)

```json
{
  "schema_version": 1,
  "kind": "lean-ctx.license",
  "customer": "LGT-Bank",
  "plan": "enterprise",
  "issued_at": "2026-06-19T00:00:00+00:00",
  "expires_at": "2027-06-19T00:00:00+00:00",
  "audit_retention_days": 2555,
  "note": "MoU-2026",
  "signer_public_key": "<hex 64>",
  "signature": "<hex 128>"
}
```

- `plan` is the granted commercial plan; its [entitlements](billing-plane-v1.md)
  (`sso_oidc`, `sso_scim`, `audit_retention_days`, …) become available offline.
- `expires_at` is optional (omit for a perpetual license).
- `audit_retention_days` is an optional negotiated override for a custom deal.
- `note` is a free-form contract reference — part of the signed payload.

## Signature construction (normative)

Mirrors the [signed savings batch](../../rust/src/core/savings_ledger/signed_batch.rs),
the [compliance report](compliance-report-v1.md) and the [org policy](org-policy-v1.md):

1. Set `signature` and `signer_public_key` to `null`.
2. `canonical = serde_json::to_vec(&license)` (serde field order; the two cleared
   fields are skipped when `None`).
3. `signature = Ed25519_sign(vendor_private_key, canonical)`.
4. Embed `signer_public_key = hex(vendor_public_key)` and `signature = hex(sig)`.

Verification clears the two fields again, recomputes `canonical`, and checks the
signature against the embedded public key — then checks that the public key is a
**trusted anchor**. Any change to a signed field (e.g. upgrading `plan` after
signing) breaks the signature.

## Trust anchor

A license is honoured only when its signer is a trusted vendor key:

- **Compiled-in** — `VENDOR_PUBLIC_KEYS` in
  [`core/license/mod.rs`](../../rust/src/core/license/mod.rs) (the shipped lean-ctx
  licensing key). This is the anchor; forging a license needs the vendor private
  key.
- **Runtime** — `LEAN_CTX_LICENSE_PUBKEY` (comma/space-separated hex) adds keys
  for a vendor running their own signing key or a test harness. The trusted set is
  the union of both.

lean-ctx is open-core, so this is **contractual** enforcement (a determined
operator can recompile), not DRM. The point is a clean, auditable, offline proof
of entitlement — not anti-piracy.

## Validity + offline grace

`active()` honours a license when it (a) verifies against a trusted anchor and
(b) is within its validity window. Expiry uses a generous offline grace
([`LICENSE_GRACE_DAYS = 30`](../../rust/src/core/license/mod.rs)) so a clock skew
or a late renewal never hard-cuts a paying customer:

| State | Effect |
|---|---|
| not expired | active |
| expired ≤ 30 days | active (within grace, flagged in `status`) |
| expired > 30 days | ignored (logged) |
| untrusted signer | ignored (logged) |
| bad signature | ignored (logged) |

Ignoring is **silent** at the resolver — the machine simply falls back to the
cloud/cached plan; it never errors out a self-host because of a license problem.

## Plan resolution (elevation)

The effective-plan resolvers fold the license in **after** the cloud/cached plan:

```text
effective = max_by_rank(cloud_or_cached_plan, license_plan)
```

[`cloud_client::elevate_with_license`](../../rust/src/cloud_client.rs) returns
`PlanSource::License` when the license wins. A higher cloud plan is never
downgraded by a lesser license, and a license never lowers a plan. Both the
cached (hot-path) and the live (logged-in) resolvers apply it, so a self-host with
a Free hosted account still unlocks Enterprise from the license.

## Pluggable source

The active artifact is resolved in order:
1. `LEAN_CTX_LICENSE` — an explicit path (containers, air-gapped installers, MDM);
2. `<config_dir>/license.json` — where `license install` writes.

## CLI

```bash
# Vendor (needs the signing key):
lean-ctx license keygen --out vendor-signing.key            # bootstrap a key
lean-ctx license issue --customer LGT-Bank --days 365 \
  --note MoU-2026 --key vendor-signing.key --out lgt.json   # mint + sign

# Customer (offline, on the self-host):
lean-ctx license verify  lgt.json        # signature + trust + expiry
lean-ctx license install lgt.json        # verify, then install
lean-ctx license status [--json]         # installed license + effective grant
lean-ctx license uninstall
```

The signing key is read from `--key` or `$LEAN_CTX_LICENSE_SIGNING_KEY` (a 32-byte
key file or a 64-char hex string).

## Invariants

1. **Local-Free** — only commercial/hosted entitlements are ever gated.
2. **Offline** — issue, install and verify need no network.
3. **Un-forgeable** — a valid license requires the vendor private key; an
   untrusted signer is ignored.
4. **Fail-open for the user** — an invalid/expired/missing license never breaks a
   self-host; it falls back to the cloud/cached plan.
5. **Monotone** — a license only ever *raises* the effective plan.

## Threat model

| Threat | Mitigation |
|---|---|
| Customer self-issues Enterprise | needs a trusted-anchor private key; arbitrary keys are not honoured |
| Edit `plan`/`expires_at` in the file | breaks the Ed25519 signature |
| Replay an expired license | `expires_at` + 30-day grace, then ignored |
| License breaks an air-gapped self-host | fail-open: any license problem ⇒ fall back, never error |
| Local feature gated by license | impossible — local plane has no entitlement checks (invariant test) |

## Module map

| File | Responsibility |
|---|---|
| [`core/license/model.rs`](../../rust/src/core/license/model.rs) | `LicenseV1` artifact + Ed25519 sign/verify + expiry math |
| [`core/license/store.rs`](../../rust/src/core/license/store.rs) | locate / read / install / uninstall (pluggable source) |
| [`core/license/mod.rs`](../../rust/src/core/license/mod.rs) | trusted anchors, `active()`/`effective_plan()` with grace, `status()` |
| [`cloud_client.rs`](../../rust/src/cloud_client.rs) | `PlanSource::License` + `elevate_with_license` in the resolvers |
| [`cli/license_cmd.rs`](../../rust/src/cli/license_cmd.rs) | `license keygen/issue/install/status/verify/uninstall` |
```
