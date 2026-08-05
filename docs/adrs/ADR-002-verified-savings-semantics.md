# ADR-002: Verified Savings Semantics

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx must distinguish locally observed or estimated savings from savings that
are sufficiently validated for contractual settlement. A single request can show
a reduction in token use, latency, or estimated provider cost, but it cannot prove
an organization-wide counterfactual. Verification requires historical workload
data, an organization-level baseline, a control or comparison population,
attribution to a specific optimization, an explicit quality policy, externally
verified prices, and a reproducible contractual methodology.

The open-source runtime has the local execution evidence needed to emit
`SavingsObservationV1`. This record is raw input: it may contain measurements and
estimates, but it is not a settlement claim. The proprietary `optimize/` domain has
the cross-workload intelligence and contractual policies required to convert
observations into a verified claim.

The required event chain is:

```text
OSS -> SavingsObservationV1 -> Intelligence -> Optimize
    -> VerifiedSavingsV1 -> Settlement
```

The architecture must also let optimization and settlement share a stable data
contract without either domain importing the other's implementation. Corrections
must preserve an auditable history rather than mutating previously issued claims.

## Decision
`VerifiedSavingsV1` is produced exclusively by the proprietary `optimize/`
domain. Open-source components MUST NOT construct, issue, or label any record as
`VerifiedSavingsV1`; they emit `SavingsObservationV1` only.

Before issuing `VerifiedSavingsV1`, `optimize/` MUST complete all of the following:

1. Calculate an organization-wide baseline rather than a single-request
   comparison.
2. Attribute the savings to the optimization or optimizations that caused it.
3. Apply the versioned quality gate and meet its confidence threshold.
4. Verify costs against an external, versioned price source effective for the
   measurement period.
5. Bind the result to a versioned methodology and methodology hash so the
   calculation is reproducible.

`VerifiedSavingsV1` is defined in a small proprietary
`verified-savings-contract/` crate containing types only (approximately 200 lines
of code). Both `optimize/` and `billing-settlement` import this contract crate.
Neither domain imports the other domain's implementation.

The complete version 1 contract is:

```rust
VerifiedSavingsV1 {
    verified_savings_id: String,
    organization_id: String,
    workload_id: String,
    provider_ids: Vec<ProviderId>,
    model_ids: Vec<ModelId>,
    baseline_cost: MoneyV1,
    actual_cost: MoneyV1,
    incremental_cost: MoneyV1,
    eligible_savings: MoneyV1,
    price_book_version: String,
    pricing_effective_at: DateTime,
    quality_status: QualityStatus,
    baseline_method: BaselineMethod,
    baseline_confidence_bps: u32,  // Basis Points (0-10000), NOT f64
    quality_policy_version: String,
    methodology_version: String,
    methodology_hash: String,
    evidence_refs: Vec<EvidenceRefV1>,
    evidence_digest: String,
    measurement_period: DateRange,
    issued_at: DateTime,
    calculation_version: String,
    status: VerifiedSavingsStatus,  // Active | Superseded | Revoked
    supersedes_id: Option<String>,
    correction_reason: Option<String>,
}
```

`baseline_confidence_bps` uses integer basis points in the inclusive range
`0..=10000`. Floating-point confidence is prohibited because serialization,
comparison, threshold evaluation, and audit reproduction must be deterministic.
All monetary values use `MoneyV1`; consumers MUST NOT infer currency or perform
settlement arithmetic using floating-point numbers.

An issued record is immutable. A correction creates a new `VerifiedSavingsV1`
with a new `verified_savings_id`, `supersedes_id` referencing the prior record,
and a non-empty `correction_reason`. The prior record is retained and transitioned
to `VerifiedSavingsStatus::Revoked`; the replacement is normally `Active`. If a
record is replaced without invalidating its historical truth, the prior record may
instead be `Superseded`. Settlement MUST use only an eligible `Active` record and
MUST follow the supersession chain to prevent duplicate settlement.

`evidence_refs`, `evidence_digest`, `methodology_version`, `methodology_hash`,
`calculation_version`, `quality_policy_version`, `price_book_version`, and
`pricing_effective_at` collectively identify the inputs and rules necessary to
audit and reproduce the claim. `quality_status` records the quality-gate outcome;
its presence does not authorize OSS or settlement to perform verification.

## Consequences
The distinction between observation and verification becomes explicit and
enforceable at the type and crate boundaries. OSS remains useful for local
measurement without exposing proprietary attribution, quality-gate, price
verification, or contractual methodology logic. Settlement receives a narrow,
versioned, reproducible input and does not depend directly on `optimize/`.

Verified claims carry sufficient provenance for audit, replay, correction, and
dispute resolution. Integer basis points and `MoneyV1` provide deterministic
threshold and monetary semantics. Append-only corrections retain historical
evidence and make duplicate or stale settlement detectable.

The design adds an intermediate contract crate and requires schema/version
governance across two proprietary consumers. Verification cannot operate from a
single local request or in an OSS-only deployment. Organization-wide data,
external price books, evidence retention, and supersession handling increase
storage and operational complexity. Changes to verification fields require a new
compatible contract version or an explicit migration.

## Alternatives Considered
**Treat local savings as verified.** Rejected because local observations lack an
organization-wide baseline, control or comparison context, attribution, a quality
gate, externally verified prices, and the contractual methodology. A local
estimate cannot support settlement.

**Place `VerifiedSavingsV1` in the OSS protocol.** Rejected because doing so would
blur the trust boundary and imply that OSS has authority to issue verified claims.
Verification depends on proprietary attribution and policy logic unavailable to
the OSS runtime. OSS retains `SavingsObservationV1` as its public output.

**Combine optimization and settlement in one crate.** Rejected because it tightly
couples savings calculation to financial settlement, expands the blast radius of
changes, and prevents independent testing and deployment. A types-only
`verified-savings-contract/` crate provides the required shared boundary without
domain coupling.

## References
- Platform Architecture Rebuild v5 (plan)
