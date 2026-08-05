# ADR-001: Protocol Scope and Versioning

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx needs a stable, open protocol that independent producers, consumers, and
offline components can share without depending on the proprietary platform
implementation. The protocol must define the data exchanged across process,
storage, and trust boundaries while remaining usable by OSS integrations under
Apache-2.0.

If the protocol crate also exposes behavior, consumers become coupled to platform
policy, verification, persistence, or transport implementations. That would make
otherwise compatible implementations depend on lean-ctx business logic and would
blur the boundary between an interoperable wire contract and proprietary product
capabilities.

Financial values require exact representation. Floating-point values cannot
reliably represent decimal currency amounts and can produce non-deterministic
totals across languages or serialization round trips. The protocol also needs an
explicit compatibility policy so persisted events and intermittently connected
clients remain readable during upgrades.

## Decision
`lean-ctx-protocol` is an OSS crate licensed under Apache-2.0. It contains only
serializable data types and normative serialization specifications. It contains
no traits, business logic, service implementations, storage implementations,
policy evaluators, or savings-verification algorithms.

The crate defines these versioned public data contracts:

- `UsageEventV1`
- `QualityEventV1`
- `SavingsObservationV1`
- `PolicyBundleV1`
- `PolicyDecisionV1`, as a result struct rather than a trait
- `ExperimentAssignmentV1`
- `EvidenceEnvelopeV1`
- `EvidenceGapV1`, including both `Opened` and `Closed` lifecycle states
- `MoneyV1`

It also defines the serializable formats and validation constraints required for
interoperability:

- event envelope format;
- schema-version identifiers;
- protocol error codes;
- idempotency-key format;
- Ed25519 signature-envelope format; and
- offline-buffer format and replay metadata.

These definitions specify what bytes and fields cross a boundary. They may include
serialization derives, enums, newtypes, constants that identify schema versions,
and structural validation constraints. They must not decide policy, perform I/O,
calculate verified savings, sign or verify messages, manage retries, or execute
offline-buffer replay.

`VerifiedSavingsV1` is explicitly excluded. A savings observation is interoperable
input and is therefore represented by `SavingsObservationV1`; a verified savings
result depends on proprietary verification logic and remains outside the OSS
protocol crate.

Protocol payloads use Serde-compatible JSON as the canonical interchange
representation. Per-concern modules organize the crate, for example `money.rs`,
`usage.rs`, `quality.rs`, `policy.rs`, `experiment.rs`, and `evidence.rs`. The
module boundary does not alter the stable serialized names.

All initially accepted schemas carry a `V1` suffix. Changes within a version are
additive only: producers may add optional fields and new explicitly extensible
enum cases where the relevant schema defines forward-compatible handling, but
must not remove fields, rename fields, change field meaning or type, alter required
field semantics, or reuse an existing discriminant for a different meaning.
Consumers must ignore unknown object fields unless a specific signed-envelope
verification rule requires retaining their exact serialized representation.

Any incompatible change creates a new type version, such as `UsageEventV2`, and a
new major protocol version. The previous major version remains supported for at
least one year after the replacement version is released. During that interval,
boundary adapters may translate versions outside `lean-ctx-protocol`; translation
logic does not belong in the protocol crate.

`MoneyV1` represents a decimal amount exactly:

```rust
pub struct MoneyV1 {
    pub currency: CurrencyCode, // ISO 4217
    pub coefficient: i128,
    pub scale: u8,
}
```

Its numeric value is `coefficient × 10^-scale` in the currency identified by
`currency`. Floating-point monetary fields are forbidden throughout the protocol.
Values retain their source precision during ingestion, aggregation, savings
observation, evidence exchange, and policy processing. Rounding to the currency's
legal minor unit occurs only when an invoice line is produced, outside the
protocol crate. Consequently, `MoneyV1` itself neither rounds nor assumes that
every currency has two minor-unit digits.

## Consequences
The protocol can be adopted independently of the lean-ctx platform and can remain
small, auditable, deterministic, and suitable for code generation or bindings in
other languages. Producers can persist and replay JSON events without linking
platform behavior, and human operators can inspect payloads directly. Explicit
versioned names make compatibility visible in APIs and stored data.

Exact decimal money avoids binary floating-point drift, preserves precision until
the legally relevant invoice boundary, and supports currencies with different
minor-unit rules. The `i128` coefficient provides a large fixed bound, so producers
must reject out-of-range values rather than silently saturating or converting to
floating point.

The additive-only rule constrains schema evolution. Breaking improvements require
parallel types, adapters, migration work, and at least one year of dual-version
support. JSON payloads may be larger and slower to parse than binary formats, but
this cost is accepted for readability and implementation simplicity.

Keeping behavior outside the crate means applications must supply their own
policy evaluation, signature operations, buffering, persistence, and verification
implementations. Conformance is defined by serialized data and normative formats,
not by sharing an implementation trait.

## Alternatives Considered
Putting traits in `lean-ctx-protocol` was rejected because traits would couple the
OSS compatibility layer to Rust-specific platform abstractions and implementation
lifecycle decisions. In particular, `PolicyDecisionV1` is portable result data;
policy evaluation behavior belongs elsewhere.

Protocol Buffers and FlatBuffers were rejected. Their compact binary encodings and
generated bindings do not justify the additional schema toolchain and reduced
human readability for the expected event volumes. Serde JSON is simpler to debug,
archive, replay, and integrate, with sufficient performance for this protocol.

A single monolithic `types.rs` was rejected because unrelated concerns would share
one change hotspot and ownership boundary. Per-concern modules make the protocol
surface easier to review and evolve while preserving one crate and one versioning
policy.

Floating-point money was rejected because it cannot guarantee exact decimal
round-trips or deterministic billing totals. Storing only legal minor units was
also rejected because observations and intermediate calculations may require more
precision than an invoice permits.

## References
- Platform Architecture Rebuild v5 (plan)
- [ISO 4217 currency codes](https://www.iso.org/iso-4217-currency-codes.html)
- [RFC 8032: Edwards-Curve Digital Signature Algorithm (EdDSA)](https://www.rfc-editor.org/rfc/rfc8032)
