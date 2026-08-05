# ADR-014: Money Precision and Usage Breakdown

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx measures provider usage at a granularity where a single observation can
cost substantially less than one legal currency minor unit. Those observations
feed `SavingsObservationV1`, verified savings, statements, invoices, and audit
evidence. If intermediate values are rounded to cents or represented as binary
floating point, aggregation order can change totals and independent replays can
disagree. Such disagreement is unacceptable for billing, settlement, and dispute
resolution.

Provider prices also distinguish input, output, cached-input, cache-write, and
reasoning tokens. Non-token modalities such as images and audio introduce other
measured units. A single token total cannot retain enough information to apply a
versioned price book correctly.

Finally, monetary and usage claims need references to supporting artifacts. A
plain URI does not state what kind of evidence it identifies, whether the
artifact has the expected content, or whether its signature was verified when
the reference was recorded.

## Decision
All monetary values in protocol types and business logic MUST use `MoneyV1`.
Binary floating-point types, including `f32` and `f64`, MUST NOT represent money,
rates, confidence values, intermediate monetary results, or monetary totals.

```rust
MoneyV1 {
    currency: CurrencyCode, // ISO 4217
    coefficient: i128,
    scale: u8,
}
```

The represented amount is `coefficient × 10^-scale`. For example, USD 0.012345
is represented as:

```rust
MoneyV1 {
    currency: CurrencyCode::USD,
    coefficient: 12_345,
    scale: 6,
}
```

The signed `i128` coefficient supports values up to approximately
`±1.7 × 10^38` before applying the scale. The `u8` scale supports 0 through 255
decimal places. These ranges exceed expected pricing and aggregation needs while
keeping the wire contract explicit and deterministic.

Every `MoneyV1` carries its own `CurrencyCode`. Parent records such as
`SavingsObservationV1` and `VerifiedSavingsV1` MUST NOT add an ambient or shared
`currency` field. A record containing several monetary fields may therefore
represent its currencies without allowing a parent currency to contradict a
child value.

Monetary arithmetic is exposed through checked methods on `MoneyV1`. Addition,
subtraction, comparison, and aggregation MUST reject operands with different
currencies. Implementations MUST align decimal scales using checked integer
arithmetic and report coefficient or rescaling overflow; they MUST NOT silently
wrap, saturate, truncate, or convert through floating point. Equivalent values
with different scales, such as coefficients `120` at scale `3` and `12` at scale
`2`, compare as equal after checked scale alignment.

Intermediate observations and business calculations retain sub-cent precision.
Rounding to the currency's legal minor unit occurs only when materializing a
statement or invoice. That boundary uses bankers' rounding (round half to even),
including for negative values. The unrounded amount and its scale remain the
authoritative input to the rounded statement or invoice amount so the result can
be reproduced.

Confidence is represented independently as integer basis points:

```rust
baseline_confidence_bps: u32 // inclusive range 0..=10_000
```

Schema validation MUST reject values above 10,000. Confidence MUST NOT use
`f64`; integer basis points make serialization, threshold comparison, and replay
deterministic.

Provider usage is recorded in `UsageBreakdownV1`:

```rust
UsageBreakdownV1 {
    input_tokens: u64,
    output_tokens: u64,
    cached_input_tokens: u64,
    cache_write_tokens: u64,
    reasoning_tokens: u64,
    other_units: Vec<MeasuredUnitV1>,
}

MeasuredUnitV1 {
    unit_type: String, // for example, "image_pixels" or "audio_seconds"
    quantity: u64,
}
```

Cached-input and cache-write tokens remain separate because providers can price
them differently from ordinary input tokens and from each other. Reasoning and
output tokens also remain distinct. All quantities use `u64`; arithmetic that
combines them MUST be checked even though overflow is not realistic for an
individual workload.

`UsageBreakdownV1` contains no derived fields such as `total_tokens`. Consumers
compute totals on read using the categories relevant to their purpose. This
prevents a stored total from becoming inconsistent with its components and
avoids falsely treating non-token `other_units` as tokens. `other_units` extends
the protocol without changing the fixed token categories; price books and
validators interpret recognized `unit_type` values explicitly and MUST NOT
silently price an unknown unit type.

Supporting artifacts are referenced through `EvidenceRefV1`:

```rust
EvidenceRefV1 {
    kind: EvidenceKind,
    uri: String,
    digest: String,
    signature_status: SignatureStatus,
}

enum EvidenceKind {
    ProviderReceipt,
    RuntimeLog,
    SignedBatch,
    QualityMeasurement,
    ExperimentOutcome,
}

enum SignatureStatus {
    Verified,
    Unverified,
    NotSigned,
}
```

`kind` is mandatory and prevents evidence from entering an unclassified generic
bucket. `uri` identifies a storage location or content address. `digest` contains
the SHA-256 digest of the referenced artifact and enables integrity comparison
without first trusting the storage location. `signature_status` records the
verification state at the time the reference is created; `Verified` means that
signature verification succeeded, `Unverified` means a signature exists but has
not been successfully verified, and `NotSigned` means no signature is asserted.
Changing the artifact or its verification state requires a new reference or
containing record rather than mutation of historical evidence.

## Consequences
Monetary aggregation and confidence evaluation are deterministic across
platforms, serialization round trips, and calculation order. Sub-cent provider
costs can accumulate without premature loss, while statement and invoice totals
have one explicit rounding boundary. Self-describing monetary fields prevent
currency drift within multi-currency records.

Granular usage preserves the inputs required for provider-specific and
versioned pricing. New measured modalities can be recorded without redefining
the fixed token fields, and omission of derived totals removes a source of stale
or contradictory data.

Typed, digested evidence references improve validation, auditability, and
tamper detection. Signature state remains explicit rather than being inferred
from the evidence kind or URI.

The design requires checked decimal arithmetic, scale alignment, overflow
handling, ISO 4217 validation, and boundary-specific rounding tests. Values with
different scales may require larger intermediate integers or explicit overflow
errors. Consumers must compute totals and understand supported measured-unit
types. SHA-256 digests prove content identity, not truthfulness, provenance, or
successful signature verification.

## Alternatives Considered
Using `f64` for money or confidence was rejected because binary floating point
cannot exactly represent many decimal values. It creates rounding drift,
order-dependent totals, non-reproducible threshold decisions, and avoidable
billing disputes.

Storing money only in legal minor units, such as cents, was rejected because
provider observations and rates require sub-cent precision. Rounding each
observation before aggregation systematically loses information.

Putting a separate currency field on a parent record was rejected because it can
contradict an embedded monetary value and does not safely support multi-currency
records. Each `MoneyV1` is self-describing.

Storing only `total_tokens` was rejected because providers price token classes
differently and token totals cannot represent image, audio, video, or future
non-token usage. Derived totals are computed from the granular source fields.

Using untyped evidence strings was rejected because a string cannot enforce
evidence categorization, bind a URI to expected artifact content, or state the
signature-verification result.

## References
- Platform Architecture Rebuild v5 (plan)
- ISO 4217: Codes for the representation of currencies
- NIST FIPS PUB 180-4: Secure Hash Standard (SHA-256)
