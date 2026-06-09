# Contract: Billing Plane v2 — Metered Add-ons (`billing-plane-v2`)

Status: stable · Plane: commercial (Team/Cloud) · Base: [`billing-plane-v1`](billing-plane-v1.md)
Source: engine `rust/src/core/billing/metering.rs` · control plane `lean-ctx-cloud/src/metering.rs`

An **additive** extension of [`billing-plane-v1`](billing-plane-v1.md): it adds a
usage-metered **hosted-index storage-overage** add-on on top of the flat plans,
without changing any plan, entitlement, or the local experience. Everything in
v1 still holds.

> Local-Free Invariant (RFC §4/§6): the Personal (local) plane is free, ungated,
> best-in-class — forever. Metering only **describes** hosted usage; it never
> gates, throttles, or bills a local capability.

## What v2 adds (over v1)

1. A second metering **dimension** alongside the v1 savings-ledger `Usage`:
   **hosted-index storage overage** — bytes stored in the hosted retrieval index
   above the plan's included `hosted_index_mb` quota. It is **server-measured**
   (the team server's `/v1/storage` report), so it needs no client signature.
2. A `metering` block on the control-plane team payloads
   (`/api/account/team/storage` and `/api/account/team/usage`), computed from the
   already-measured storage figures + a configurable rate.
3. A **metered Stripe price** (`usage_type=metered`, `aggregate_usage=
   last_during_period`) for the storage-overage dimension, provisioned by
   `stripe-setup.py --storage-metering`.

## Rollout: display-first (opt-in, no surprise bills)

Metering ships **visibility-first**. The rate lives in the control plane env
`LEANCTX_BILLING_STORAGE_OVERAGE_CENTS_PER_GB` (cents per GB / month):

- **Unset / `0`** ⇒ `billing_active = false`. Usage, quota headroom, and the
  threshold state are surfaced; **no projected cost is shown and nothing is
  billed**. `stripe-setup.py --storage-metering` refuses to invent a price.
- **Positive rate** ⇒ `billing_active = true`. A *projected* monthly cost is
  shown for any overage, clearly labelled "estimated · not yet billed" until the
  metered usage-record push is enabled (a deliberate follow-up).

## The `metering` block (`camelCase`)

Carries only numbers — no paths, prompts, or content — so it is safe to surface
and to reconcile against billing.

```json
{
  "usedBytes": 6000000000,
  "quotaBytes": 5000000000,
  "overageBytes": 1000000000,
  "percent": 120.0,
  "unlimited": false,
  "state": "over",
  "rateCentsPerGb": 50,
  "billingActive": true,
  "projectedCostCents": 50
}
```

- `state` ∈ `none | ok | warn (≥50%) | critical (≥80%) | over (≥100%) | unlimited`.
- `quotaBytes` is `null` and `overageBytes`/`projectedCostCents` are `0` for an
  **unlimited** (Enterprise) quota.
- Billing convention: 1 GB = 1e9 bytes (decimal), matching Stripe metered units.

## Invariants (test-enforced)

All of `billing-plane-v1`'s invariants, plus
(`lean-ctx-cloud/src/metering.rs` tests):

1. **`0` (none) is never conflated with `UNBOUNDED` (unlimited)** — a `0` quota
   yields `state = "none"` with no cost; an unlimited quota yields
   `state = "unlimited"` with no overage.
2. `overageBytes = max(0, used − quota)`; unlimited ⇒ `0`.
3. `projectedCostCents = 0` (and is suppressed) whenever `billingActive = false`
   — display-first never bills.
4. The `metering` block is privacy-preserving (numbers only).
5. Only `signed && chain_valid` savings-derived usage is ever billable
   (unchanged from v1 `Usage::is_billable`); the storage dimension is
   server-measured and additive.
6. Nothing in the metering path gates a local feature (Local-Free preserved).

## Versioning

## Meter Events (Stripe Billing Meters API)

Usage is pushed via the Stripe Billing Meters API (`POST /v1/billing/meter_events`),
not the legacy subscription-item usage records (removed in Stripe 2025-03-31.basil).
The billing service runs an hourly background job (`metering_job`) that, for each
active team account with a provisioned server and control token:

1. Fetches `/v1/storage` from the team server.
2. Persists a `billing_storage_samples` row (usage trend + audit).
3. Checks threshold crossings (50/80/100%) and sends an idempotent email alert
   (one per threshold per billing period, via SMTP/ZeptoMail).
4. If overage > 0 and a Stripe Billing Meter is configured, pushes a meter event
   with `event_name = leanctx_hosted_index_storage_gb` and `value` = overage in
   GB (rounded up to 0.01 GB).

## Versioning

Named `v2` because it introduces a new **metered add-on surface** (a billable
dimension + the `metering` block + a metered price + meter events), even though
it is additive and changes no v1 plan/entitlement or local-free semantics.
Adding further metered dimensions (connector sync volume, retrieval queries)
under the same display-first, signed/server-measured, Local-Free rules stays
`v2`.
