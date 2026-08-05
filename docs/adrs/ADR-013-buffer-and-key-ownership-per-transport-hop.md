# ADR-013: Buffer and Key Ownership per Transport Hop

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx transports signed evidence from an OSS Runtime to an Enterprise Sidecar,
then from the Sidecar to the proprietary Platform. These are separate transport
hops with different availability, durability, and trust requirements. Treating
them as one pipeline would distribute durable-storage and encryption-key custody
across components that do not need it.

The Runtime is latency-sensitive and must not become a durable metering store. It
needs only enough buffering to absorb short Sidecar stalls. The Sidecar, by
contrast, must survive process restarts and Platform outages without discarding
accepted evidence such as `SavingsObservationV1`. It therefore needs durable,
encrypted local storage and retry behavior.

Failure reporting must remain possible when the Runtime's normal buffer is full.
A loss marker stored in that same buffer could be lost for the reason it is meant
to report. Key ownership must also follow least privilege: the Runtime does not
need a data-encryption key (DEK), and the Platform must not supply a Sidecar spool
DEK on demand. The governing security gate is: a DEK is never stored unprotected
beside the buffer it encrypts.

Earlier plan versions proposed sending a wrapped DEK to the Runtime and described
the Runtime as owning an encrypted disk spool. Those proposals are superseded by
this decision.

## Decision
Buffering and key ownership are defined independently for each transport hop:

```text
Runtime
  volatile ring buffer --signed event envelopes--> Sidecar
  reserved Gap-Journal

Sidecar
  encrypted durable spool --signed batches-------> Platform
  Sidecar-controlled KMS/Vault
```

### Hop 1: Runtime to Sidecar

The Runtime owns a small, bounded, in-memory ring buffer. Its capacity is
configurable; 10 MiB is the default deployment target. The ring buffer is
volatile and is discarded on Runtime process termination or host restart. It is
not mirrored to disk and is not a persistent spool.

The Runtime does not receive, derive, cache, or use a buffer DEK. Encryption at
rest is inapplicable to the volatile ring buffer. Transport confidentiality is
provided by the selected `RuntimeSidecarTransport`, while application-level
integrity and provenance are provided by signed event envelopes.

Each event envelope is signed with the Runtime's Ed25519 Instance Identity and
contains the tenant, source instance, sequence, protocol version, payload type,
and payload. Payloads include concrete protocol records such as
`SavingsObservationV1`; monetary values within such records use `MoneyV1`.

```rust
SignedEventEnvelopeV1 {
    tenant_id,
    source_instance_id,
    sequence: u64,
    protocol_version,
    payload_type,
    payload,
    signature: Ed25519Signature, // Runtime Instance Identity
}
```

If the ring buffer is full, the Runtime opens an evidence gap by writing an
`EvidenceGapOpenedV1` marker to the Gap-Journal. If the Sidecar is unreachable
beyond a configurable timeout, the Runtime performs the same action. The marker
identifies the applicable reason, including `BufferFull` or
`SidecarUnreachable`, and follows the evidence-gap lifecycle defined by ADR-003.

The Gap-Journal is a separately reserved, pre-allocated, fixed 1 MiB area owned
by the Runtime. It is not part of the ring buffer and is never available for
ordinary event envelopes. Gap markers are self-signed with the Runtime Instance
Identity. The write path for `EvidenceGapOpenedV1` remains available when the
main ring buffer is at capacity; exhaustion or corruption of the Gap-Journal is
a catastrophic operational condition and must raise a critical alert.

The Gap-Journal is the sole intentional persistent exception to the statement
that the Runtime has no durable event buffer. It contains gap lifecycle metadata,
not queued copies of normal events, and does not require or justify giving the
Runtime a spool DEK.

### Hop 2: Sidecar to Platform

The Sidecar owns an encrypted durable spool persisted on local disk. Once the
Sidecar accepts an event envelope for durable delivery, it writes the event to
the spool before acknowledging durable acceptance. The spool survives Sidecar
restart and is bounded by an explicit disk allocation.

Spool segments are encrypted at rest with DEKs obtained from the Sidecar's own
KMS or Vault integration. The Sidecar rotates the DEK when it rotates a spool
segment. A segment records a non-secret key identifier and the metadata required
to request or unwrap its DEK, but never stores an unprotected DEK beside its
ciphertext. The Runtime never receives these DEKs. The Platform does not deliver
them directly in the request path and does not need custody of them to accept a
batch.

The Sidecar packages persisted event envelopes into batches and signs each batch
with the Sidecar Identity. The original Runtime signatures remain attached so
the Platform can verify event provenance independently of batch provenance.

```rust
SignedEvidenceBatchV1 {
    sidecar_id,
    tenant_id,
    first_sequence: u64,
    last_sequence: u64,
    events: Vec<SignedEventEnvelopeV1>,
    idempotency_key,
    signature: Ed25519Signature, // Sidecar Identity
}
```

When the Platform is unreachable, the Sidecar retains batches locally and
retries with exponential backoff and stable idempotency keys. It does not discard
the oldest event merely to make a failed delivery appear successful. If an event
remains undelivered beyond the configured `max_event_age`, the Sidecar initiates
the evidence-gap protocol and raises the required alert. Disk-allocation
exhaustion likewise becomes an explicit evidence gap rather than silent loss.

The following ownership matrix is normative:

| Asset | Runtime | Sidecar | Platform |
|---|---|---|---|
| Volatile Runtime ring buffer | Owns | No access required | No access |
| Runtime Gap-Journal | Owns and writes | May forward markers | Receives markers |
| Runtime Instance Identity private key | Owns | Does not own | Does not own |
| Encrypted durable spool | No access | Owns | No direct access |
| Spool DEKs | Never receives | Obtains from own KMS/Vault | Never supplies directly |
| Sidecar Identity private key | No access | Owns | Does not own |
| Runtime public verification identity | Publishes | Verifies/forwards | Verifies |
| Sidecar public verification identity | No private-key access | Publishes | Verifies |

Compromise boundaries follow this ownership. Compromising the Runtime does not
expose spool DEKs, Sidecar or Platform private signing keys, or historical events
already durably accepted and removed from Runtime memory. The Runtime's own
Instance Identity is necessarily within its trust boundary and must be revoked
and rotated after compromise. Compromising the Sidecar may expose plaintext of
the currently accessible spool while the Sidecar can obtain its DEKs, despite
encryption at rest; it does not expose Runtime or Platform private keys.
Compromising the Platform does not, by itself, expose Sidecar spool DEKs because
the Sidecar KMS/Vault is an isolated key domain.

## Consequences
Positive consequences:

- The Runtime remains small and latency-oriented, with no disk-spool lifecycle,
  spool encryption, or buffer-DEK management.
- Durable evidence survives Platform and Sidecar process outages once accepted
  into the Sidecar spool.
- Separate Runtime and Sidecar signatures preserve end-to-end event provenance
  and hop-level batch accountability.
- A full Runtime buffer can still produce a signed `EvidenceGapOpenedV1` because
  the 1 MiB Gap-Journal has independently reserved capacity.
- KMS/Vault isolation prevents a Platform compromise from automatically exposing
  Sidecar spool DEKs and prevents a Runtime compromise from decrypting the spool.
- Per-segment DEK rotation limits the ciphertext affected by one DEK and permits
  old keys to be retired after segment delivery and retention expiry.

Negative consequences:

- Runtime restart can lose events that have not yet reached the Sidecar; this is
  represented as an evidence gap rather than hidden by a false durability claim.
- The Sidecar must implement crash-safe segment writes, encryption, KMS/Vault
  integration, rotation, disk quotas, retry backoff, and idempotent delivery.
- The 1 MiB Gap-Journal is finite and requires monitoring, compaction or archival
  rules consistent with append-only gap semantics.
- Operators must provision enough Sidecar disk for the expected Platform outage
  window and choose `max_event_age` explicitly.
- Encryption at rest does not protect spool plaintext from a fully compromised,
  running Sidecar that is authorized to obtain the relevant DEKs.

## Alternatives Considered
**A persistent encrypted buffer in the Runtime.** Rejected because there is no
use case requiring the Runtime to be the durable delivery boundary. It would add
disk recovery, encryption, rotation, and KMS integration to the OSS Runtime and
widen its compromise domain. The earlier wrapped-DEK-to-Runtime design is
therefore removed.

**A DEK shared by the Runtime and Sidecar.** Rejected because it violates least
privilege and couples two compromise domains. The Runtime has no persistent
buffer to encrypt, so possession of the spool DEK would provide risk without a
required capability.

**Platform-issued Sidecar spool DEKs in the delivery request path.** Rejected
because Platform availability would then be required to create or rotate local
spool segments, and Platform compromise could expose buffer keys. The Sidecar's
own KMS/Vault is the authoritative DEK source.

**No Sidecar buffer and direct forwarding to the Platform.** Rejected because a
Platform outage would immediately cause evidence loss or push durable buffering
back into the Runtime. A durable Sidecar spool is required to decouple local
collection from Platform availability.

**Storing gap markers in the main Runtime buffer.** Rejected because buffer
exhaustion is itself a gap trigger. Failure reporting must not depend on the
resource whose failure it reports, so the Gap-Journal has separately reserved
capacity.

## References
- Platform Architecture Rebuild v5 (plan)
- ADR-003: Evidence Gaps and Non-Billable Periods
- ADR-004: Sidecar Trust and Failure Modes
- RFC 8032: Edwards-Curve Digital Signature Algorithm (EdDSA)
- NIST SP 800-57 Part 1 Rev. 5, Recommendation for Key Management
