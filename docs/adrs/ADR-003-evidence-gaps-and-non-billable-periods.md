# ADR-003: Evidence Gaps and Non-Billable Periods

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx produces evidence events that support usage accounting, savings
observations, billing, and later audit. Events are ordered per tenant and source
instance by a monotonically increasing sequence. A missing sequence range means
that the platform cannot prove which events, including billable events or
`SavingsObservationV1` records, should have occupied that range.

Evidence can be lost when the normal event buffer is full, a network timeout
prevents delivery, a process crashes before buffered events are persisted, the
sidecar is unreachable, or local storage is full. The failure that loses evidence
may also disable the mechanism normally used to report failures. In particular,
recording a loss marker in the normal event buffer is unsafe because buffer
exhaustion is itself a cause of evidence loss.

The system must therefore distinguish a verifiable, complete billing period from
a period whose evidence cannot be proven complete. Loss of evidence must never
become invisible. The audit guarantee is consequently "tamper-evident with
explicit evidence gaps," not an assertion that the evidence stream is
`lückenlos` (gapless).

## Decision
Evidence gaps are represented by two distinct, append-only lifecycle events:
`EvidenceGapOpenedV1` and `EvidenceGapClosedV1`. Neither event may be updated or
deleted after it is appended. Closure references the opening event by `gap_id`.

The versioned event payloads are:

```rust
EvidenceGapOpenedV1 {
    gap_id,
    tenant_id,
    source_instance_id,
    first_missing_sequence: u64,
    last_missing_sequence: Option<u64>, // None while open
    started_at,
    detected_at,
    reason: GapReason, // BufferFull | NetworkTimeout | ProcessCrash |
                       // SidecarUnreachable | DiskFull
    affected_event_types: Vec<String>,
    runtime_version,
    previous_evidence_hash,
    signature: Ed25519Signature,
}

EvidenceGapClosedV1 {
    gap_id, // references EvidenceGapOpenedV1
    last_missing_sequence: u64,
    ended_at,
    resolved_at,
    total_missing_events: u64,
    billing_period_status: BillingPeriodStatus, // Incomplete | NonBillable
    signature: Ed25519Signature,
}
```

`EvidenceGapOpenedV1.first_missing_sequence` is the first sequence that cannot be
accounted for. Its `last_missing_sequence` must be `None` while loss may still be
occurring. `started_at` identifies when the missing interval began, while
`detected_at` identifies when the runtime detected and journaled it. The event
records the best available `affected_event_types`; this field is diagnostic and
does not narrow the billing impact when completeness cannot be proven.

A gap may be closed only after recovery has established a definitive upper bound
for the missing sequence interval. `EvidenceGapClosedV1.last_missing_sequence`
is inclusive and must be greater than or equal to the opening event's
`first_missing_sequence`. Its count must satisfy:

```text
total_missing_events =
    last_missing_sequence - first_missing_sequence + 1
```

`ended_at` identifies when evidence loss stopped; `resolved_at` identifies when
the bound was established and the closure event was emitted. An uncertain or
still-growing interval remains open. A later observation that the estimate was
wrong must produce another append-only corrective event under a future schema;
it must not mutate either V1 event.

Each runtime reserves a 1 MiB Gap-Journal area, pre-allocated independently of
the normal event buffer. The reserved area:

- is never used for normal evidence events;
- accepts gap lifecycle markers with priority over normal event writes;
- remains available when the normal buffer is exhausted; and
- stores self-signed gap markers using the runtime's Ed25519 identity.

The opening event includes `previous_evidence_hash` to bind the marker to the
last known evidence-chain state. Both lifecycle events carry an
`Ed25519Signature`. Verification establishes marker integrity and provenance; it
does not recreate the missing evidence or make the affected interval complete.

The lifecycle is:

1. Buffer exhaustion, network timeout, process crash, unreachable sidecar, or
   disk exhaustion causes an `EvidenceGapOpenedV1` marker to be written to the
   reserved Gap-Journal.
2. While loss continues or its end is unknown, `last_missing_sequence` remains
   `None` and the gap remains open.
3. Recovery reconciles persisted and produced sequence numbers and determines
   the inclusive, bounded missing interval.
4. The system appends `EvidenceGapClosedV1`, records `total_missing_events`, and
   assigns the affected billing period a terminal evidence status.

Any billing period intersecting an open gap or a closed gap is not complete. Its
`BillingPeriodStatus` must be `Incomplete` while classification or remediation is
pending, or `NonBillable` when charges cannot be supported by complete evidence.
It must never be promoted to a complete or billable status merely because the gap
was closed: closure bounds and acknowledges the loss but does not recover the
missing events. Billing calculations involving concrete types such as `MoneyV1`
must not charge amounts derived from an affected non-billable period.

Opening a gap must emit a critical alert to both Console and Monitoring. The
alert remains active while the gap is open. Closure resolves the active-gap alert
but retains the gap, its billing effect, signatures, and alert history for audit.

## Consequences
Positive consequences:

- Evidence loss is explicit, signed, attributable to a tenant and source
  instance, and retained in the append-only audit history.
- Open-ended failures are modeled honestly without inventing an end sequence.
- A dedicated Gap-Journal can record the failure even when the normal event
  buffer caused the failure.
- Billing fails safely: unsupported charges are prevented, while
  `BillingPeriodStatus` communicates whether investigation is pending or the
  period is definitively non-billable.
- Auditors can verify the evidence chain up to the gap, the signed declaration of
  loss, and the bounded interval after recovery.

Negative consequences:

- Every runtime reserves an additional 1 MiB and must implement a separate,
  higher-priority persistence path.
- Recovery requires sequence reconciliation before a gap can close.
- Operations must manage critical alerts and potentially delayed or forgone
  revenue for affected billing periods.
- Self-signing protects marker integrity but cannot prove the contents of missing
  events or restore records such as `SavingsObservationV1`.
- Gap-Journal exhaustion remains a catastrophic condition requiring separate
  operational handling; the reserved area reduces this risk but cannot eliminate
  finite-storage limits.

## Alternatives Considered
A single `GapV1` event was rejected. At detection time the final missing sequence
and total count are commonly unknown. Updating a single event later would violate
append-only audit semantics, while delaying it would make an active gap invisible.

Silent gap handling, including logging only to ordinary operational logs, was
rejected. It violates the requirement that loss of evidence never become
invisible and could allow unsupported billing to appear fully evidenced.

Storing gap markers in the main event buffer was rejected. `BufferFull` is a
defined `GapReason`; a marker that depends on the exhausted resource cannot
reliably report that resource's failure. A separately pre-allocated Gap-Journal
is required.

Treating a closed gap as restored completeness was rejected. Closure proves only
that the missing sequence interval is bounded. It does not reconstruct or verify
the missing evidence and therefore cannot restore billability.

## References
- Platform Architecture Rebuild v5 (plan)
- RFC 8032: Edwards-Curve Digital Signature Algorithm (EdDSA)
