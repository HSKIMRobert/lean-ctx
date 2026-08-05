# ADR-011: Sidecar Transport Abstraction

## Status
Accepted

## Date
2026-08-05

## Context
The lean-ctx Runtime exchanges signed event envelopes, policies, and experiment
assignments with the Enterprise Sidecar. Deployments place those components in
different topologies: processes on one Linux or macOS host, processes on Windows,
separate containers or pods reachable through localhost, or distinct hosts across
a network. No single connection mechanism is both operationally minimal for
same-host deployments and sufficiently authenticated for every network hop.

Transport choice must not change protocol security or delivery semantics. In
particular, local IPC is not a reason to omit tenant binding, replay protection,
signatures, idempotency, or backpressure. Conversely, certificate lifecycle and
TLS overhead are unnecessary when an operating system can strongly authenticate a
same-host peer.

The two communication hops also have different durability and key-custody needs.
The Runtime must remain small and must not receive the Sidecar's storage keys. The
Sidecar must survive Platform outages without losing accepted metering evidence.
The architecture therefore needs one application-facing interface, deterministic
topology selection, and explicit ownership of buffers and signing identities.

## Decision
The Runtime uses a single asynchronous `RuntimeSidecarTransport` trait. Runtime
code above this boundary does not branch on sockets, pipes, containers, or remote
networking:

```rust
#[async_trait]
pub trait RuntimeSidecarTransport: Send + Sync {
    async fn send_envelope(
        &self,
        envelope: SignedEnvelope,
    ) -> Result<Ack, TransportError>;

    async fn receive_policy(&self) -> Result<SignedPolicy, TransportError>;

    async fn receive_assignment(
        &self,
    ) -> Result<ExperimentAssignmentV1, TransportError>;

    async fn health_check(&self) -> TransportHealth;
}
```

Implementations are selected in this strict order:

1. Select `UnixDomainSocket` when the configured UDS path exists and the host OS
   supports Unix-domain sockets. Authenticate the peer with OS credentials and
   enforce socket ownership and permissions.
2. Otherwise, select `WindowsNamedPipe` when a named pipe is configured on
   Windows. Authenticate and authorize access with the pipe ACL and Windows peer
   identity.
3. Otherwise, select `LocalhostMTLS` when the Sidecar is reached through localhost
   but is isolated in a separate container or pod, so same-host OS peer
   credentials are unavailable or do not cross the isolation boundary.
4. Otherwise, select `NetworkMTLS`. Mutual TLS is mandatory for every remote or
   otherwise non-local network hop.

Selection must fail closed when the selected transport cannot establish its
required authentication. It must not silently downgrade from mTLS to plaintext,
from OS-authenticated IPC to unauthenticated IPC, or from a configured local
endpoint to an unrelated remote endpoint. Deployment configuration and selection
outcomes must be observable without exposing credentials.

The variants are:

```text
RuntimeSidecarTransport
+-- UnixDomainSocket + OS Credentials    (same host, Linux/macOS)
+-- WindowsNamedPipe + ACL               (same host, Windows)
+-- LocalhostMTLS                        (separate containers/pods)
+-- NetworkMTLS                          (remote sidecar, mandatory)
```

All variants carry the same versioned, `serde_json`-serialized application
messages and provide these transport-independent guarantees:

- **Replay protection:** signed envelopes contain monotonically increasing
  sequence numbers whose scope and acceptance window are validated by the
  receiver.
- **Tenant binding:** the tenant identity is part of the signed envelope and is
  checked against the authenticated connection identity; routing metadata alone
  cannot establish tenant identity.
- **Signed envelopes:** the Runtime signs event envelopes with its Ed25519
  Instance Identity. Receivers verify the signature before accepting payloads.
- **Idempotency:** operations carry idempotency keys so reconnects and retries do
  not duplicate accepted effects. An `Ack` identifies acceptance of the relevant
  idempotency key and sequence state.
- **Backpressure:** implementations signal saturation explicitly through `Ack`,
  `TransportError`, and `TransportHealth`; producers bound admission or retry
  according to that signal rather than creating an unbounded queue.

Sequence numbers and idempotency keys serve different purposes: sequence numbers
detect replay and ordering violations, while idempotency keys deduplicate valid
retries across reconnects. Transport authentication supplements, but does not
replace, Ed25519 envelope verification.

Buffer and key ownership is defined per hop.

**Hop 1: Runtime to Sidecar.** The Runtime maintains only a small, bounded,
volatile in-memory ring buffer. It requires no data-encryption key because it is
not persisted. The Runtime signs each event envelope with its Instance Identity
before transmission. Backpressure may fill this ring buffer; it must not cause an
unbounded memory allocation. If the Sidecar remains unreachable beyond a
configurable timeout, the Runtime emits `EvidenceGapOpenedV1` to record the start
of the evidence discontinuity.

**Hop 2: Sidecar to Platform.** After accepting Runtime envelopes, the Sidecar
writes them to an encrypted durable spool before export. The spool's DEK is
obtained by the Sidecar from KMS or Vault and is never supplied by the Runtime.
The Sidecar constructs and signs export batches with its distinct Sidecar
Identity. When the Platform is unavailable, the Sidecar retains accepted data,
applies bounded exponential backoff, and continues subject to spool capacity and
retention controls. Data older than the configured `max_event_age` opens an
evidence gap rather than being represented as continuously observed evidence.

An `Ack` on Hop 1 means the Sidecar has accepted responsibility under its durable
spool contract; it does not mean the Platform has ingested the event. This
boundary prevents the Runtime's volatile buffer from being treated as durable
storage and keeps Platform-outage handling in the Sidecar.

The wire protocol uses framed JSON with explicit message and schema versions.
`serde_json` is sufficient because messages already use versioned domain types
such as `SignedEnvelope`, `SignedPolicy`, `ExperimentAssignmentV1`, and
`EvidenceGapOpenedV1`. Every transport implementation must pass a common
conformance suite covering serialization, authentication failure, tenant
mismatch, replay, retry/idempotency, backpressure, and reconnect behavior.

## Consequences
Runtime and Sidecar application logic depend on one stable interface while each
deployment uses its strongest practical local authentication primitive. Same-host
Unix and Windows installations avoid unnecessary certificate management and TLS
overhead, while container and remote deployments receive mandatory mutual
authentication and encryption.

Security properties remain consistent across transports. A change in topology
does not remove signed tenant binding, replay detection, idempotency, or
backpressure. Distinct Runtime and Sidecar identities preserve provenance across
the two hops, and the Sidecar's DEK remains outside the Runtime compromise domain.

The durability boundary is explicit: the Runtime can lose only its bounded
volatile buffer during failure, while the Sidecar owns durable encrypted retry.
`EvidenceGapOpenedV1` makes loss or excessive delay visible instead of silently
presenting incomplete data as continuous evidence.

The design requires four implementations, topology detection, cross-platform
tests, certificate provisioning for both mTLS variants, and a shared conformance
suite. JSON framing uses more bytes and CPU than a compact binary protocol.
Automatic selection can also obscure configuration mistakes, so selection and
authentication failures require clear diagnostics and must never trigger an
insecure fallback.

## Alternatives Considered
**A single mTLS transport.** Rejected because UDS peer credentials and Windows
named-pipe ACLs are faster and operationally simpler for same-host communication.
Requiring certificates for every local installation would add rotation and
provisioning failure modes without improving the application-envelope guarantees.

**gRPC.** Rejected because it adds a comparatively heavy HTTP/2, protobuf, code
generation, and runtime dependency surface. The required request, response,
stream-like receive, health, and backpressure semantics are small, and framed
`serde_json` messages over the selected authenticated transport are sufficient.

**A custom binary protocol.** Rejected because it would require bespoke codecs,
inspection tools, compatibility rules, and debugging support. Versioned JSON is
human-inspectable, works with standard tooling, and is adequate for the bounded
Runtime-to-Sidecar message flow; envelope signatures protect its canonical signed
representation independently of the transport framing.

## References
- Platform Architecture Rebuild v5 (plan)
- RFC 8446, The Transport Layer Security (TLS) Protocol Version 1.3
- RFC 8032, Edwards-Curve Digital Signature Algorithm (EdDSA)
- Microsoft, Named Pipe Security and Access Rights
- Linux `unix(7)` and macOS Unix-domain socket documentation
