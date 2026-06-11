# Contract: Billing Plane v3 — Business Plan (`billing-plane-v3`)

Status: stable · Plane: commercial (Team/Cloud) · Base: [`billing-plane-v1`](billing-plane-v1.md)
Source: engine `rust/src/core/billing/plans.rs` · control plane `lean-ctx-cloud/src/plan.rs`

An **additive** extension of [`billing-plane-v1`](billing-plane-v1.md) (GL #460/#533):
it adds the self-serve **`business` plan** and the **`sso_oidc`** entitlement key.
Per v1's own versioning rule ("adding a plan or entitlement field is additive"),
the semantics stay v1; this document exists because the v1 doc is frozen and the
addition deserves its own normative record. Everything in v1 and
[v2 (metered add-ons)](billing-plane-v2.md) still holds.

> Local-Free Invariant (RFC §4/§6): unchanged. `business` only adds *hosted*
> capabilities; nothing local is gated on any plan.

## What v3 adds (over v1)

1. **`business` plan** — self-serve governance at **$149/mo flat** ($1490/yr),
   no sales motion. The plan ladder becomes:
   `free ⊂ supporter ⊂ pro ⊂ team ⊂ business ⊂ enterprise`.
2. **`sso_oidc` entitlement key** — self-serve org SSO via OIDC (GL #482).
   Distinct from `sso_scim` (SAML SSO + SCIM provisioning), which stays the
   negotiated Enterprise surface.

## Catalog delta

| Entitlement | team | **business** | enterprise |
|-------------|------|--------------|------------|
| billing model | $18/seat/mo | **$149/mo flat** | negotiated |
| seats | 25 (per-seat) | **50 (flat)** | unlimited |
| hosted_index_mb | 5000 | **20000** | unlimited |
| managed_connectors | 5 | **10** | unlimited |
| private_registry | yes | **yes** | yes |
| **sso_oidc (new)** | no¹ | **yes** | yes |
| sso_scim | no | **no** | yes |
| audit_retention_days | 90 | **365** | 3650 |
| revenue_share | yes | **yes** | yes |
| supporter / cloud_sync | yes | **yes** | yes |

¹ Catalog-wise Team has no SSO. Orgs that configured OIDC while it was
Team-gated (pre-#533) are **grandfathered at the enforcement edge**: the
control plane keeps existing, already-configured SSO working for them, but new
SSO setup requires `sso_oidc` (Business or Enterprise).

All other plans gain `sso_oidc: false` — a pure schema addition; no existing
value changed. The golden fixture
(`docs/contracts/billing-plane-v1-catalog.json`, mirrored in
`lean-ctx-cloud/contracts/`) carries the new key for every plan and the new
`business` row; the cross-repo drift tripwire (GL #462) pins both sides.

## `entitlement_allows` / `min_plan_for`

- `entitlement_allows(plan, "sso_oidc")` resolves from the catalog:
  `business` and `enterprise` only.
- `min_plan_for("sso_oidc") == Some(Business)` — upgrade hints (#346) point to
  the self-serve checkout (`lean-ctx cloud upgrade --plan business`), never to
  sales.
- `min_plan_for("sso_scim") == Some(Enterprise)` — unchanged.

## Wire ids

`business` (alias `biz`) parses to `Plan::Business`; unknown ids still map to
`free` (fail-open, never gates). The id is stable and appears in checkout
(`POST /api/billing/checkout {"plan": "business"}`), webhook plan mapping
(`STRIPE_PRICE_BUSINESS_MONTHLY` / `_YEARLY`), entitlement payloads and the CLI
(`lean-ctx billing entitlements business`).

## Invariants (test-enforced)

1. All v1 invariants (local-free, additive ladder, privacy) — unchanged.
2. `business` sits strictly between `team` and `enterprise`:
   more seats/quota/retention than Team, less than Enterprise, `sso_oidc`
   without `sso_scim` (`business_is_team_plus_self_serve_governance`).
3. Catalog fixtures match byte-for-byte on both repos
   (`catalog_matches_golden_fixture`, engine + control plane).

## Versioning

Future additive plan/entitlement changes append to this ladder under the same
rule. Removing/renaming a field or changing local-free semantics requires a new
major contract version.
