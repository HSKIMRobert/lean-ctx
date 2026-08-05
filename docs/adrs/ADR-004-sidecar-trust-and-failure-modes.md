# ADR-004: Sidecar Trust and Failure Modes

## Status
Accepted

## Date
2026-08-05

## Context
The open-source lean-ctx Runtime must exchange identity, policy, experiment, and
metering information with the proprietary Platform without collapsing their
security and licensing boundary. Runtime deployments range from a single host to
separate containers, Kubernetes pods, and remote hosts. The communication design
must therefore support different operating-system primitives while preserving the
same authentication, integrity, replay, and tenant-isolation guarantees.

The Runtime is part of the data path for local context optimization and must remain
available when the Platform or Enterprise Sidecar is unavailable. However, some
Platform-issued policies protect security, compliance, or commercial controls and
cannot safely be treated as optional after they expire. Other policies can tolerate
temporary staleness in exchange for availability. A uniform failure mode would
either block too much local functionality or fail open for controls that require
strict enforcement.

The Enterprise Sidecar also buffers outbound metering data. Key custody must be
explicit: compromising the OSS Runtime must not grant authority to sign Platform
policies, access customer private keys, sign invoices or settlements, or decrypt
the Sidecar-owned metering buffer.

## Decision
The Enterprise Sidecar is the sole communication bridge between the OSS Runtime
and the proprietary Platform. The Runtime does not connect directly to Platform
APIs. The Sidecar performs:

- policy and experiment-assignment synchronization;
- workload and tenant identity propagation; and
- durable, retryable metering export.

Communication uses the `RuntimeSidecarTransport` abstraction. Deployment
detection selects one of these transports:

```text
RuntimeSidecarTransport
+-- UnixDomainSocket + OS Credentials    (same host, Linux/macOS)
+-- WindowsNamedPipe + ACL               (same host, Windows)
+-- LocalhostMTLS                        (separate containers/pods)
+-- NetworkMTLS                          (remote sidecar, mandatory)
```

`UnixDomainSocket` authenticates the peer using operating-system credentials and
socket ownership/permissions. `WindowsNamedPipe` uses pipe ACLs and authenticated
Windows identities. `LocalhostMTLS` is used when process isolation prevents use of
same-host IPC credentials, including separate containers or pods sharing a network
namespace. `NetworkMTLS` is mandatory whenever traffic can leave the local host;
plaintext or server-authentication-only remote transport is not permitted.

All transport variants carry the same signed application envelope and enforce the
same transport-independent controls:

- a tenant identifier is bound into the signed envelope and checked against the
  authenticated session identity;
- monotonically increasing sequence numbers, scoped to the authenticated session,
  provide replay detection;
- Platform-originated policies and assignments are signature-verified by the
  Runtime before use;
- idempotency keys make retried policy, assignment, and metering operations safe;
  and
- protocol/version fields permit rejection of incompatible envelopes before their
  payloads are applied.

The transport must not infer tenant identity solely from routing metadata or a
socket path. An envelope whose tenant binding disagrees with the authenticated
peer or session is rejected. Sequence numbers are not substitutes for idempotency
keys: sequence numbers reject replay within a session, while idempotency keys
deduplicate legitimate retries across reconnects and credential renewal.

Each policy carries two independent classifications:

```text
PolicyCriticality: Critical | High | Medium | Low
ExpiryBehavior:    FailClosed | FailOpen | GracePeriod
```

The signed policy explicitly specifies its criticality, expiry time, expiry
behavior, and, for `GracePeriod`, the configured grace duration. The Runtime uses
the last successfully verified policy until expiry, then applies its declared
behavior:

- `Critical` plus `FailClosed` blocks the policy-protected operation immediately
  when the policy expires. It does not terminate the Runtime or block unrelated
  local optimization.
- `FailOpen` permits the protected operation to continue with the expired policy
  state and raises an alert. `Low` plus `FailOpen` is the standard availability-
  favoring case.
- `GracePeriod` permits the protected operation only until the signed grace window
  ends, then applies enforcement. The grace duration is policy configuration, not
  an unrestricted Runtime override.

Criticality communicates impact and drives alert priority; expiry behavior is the
authoritative enforcement instruction. Invalid signatures, tenant mismatches,
replayed sequence numbers, and envelopes outside their validity interval are
treated as unavailable policy input and never refresh the cached policy.

The Sidecar may send the Runtime only the keying and control material required for
verification and short-lived session establishment:

- public verification keys for Platform policy signatures;
- short-lived session credentials used for mTLS establishment and renewal; and
- signed policies and signed experiment assignments.

The Sidecar never sends the Runtime:

- private policy or envelope signing keys, which remain in Platform KMS/HSM;
- long-term root keys;
- customer-owned private keys;
- settlement or invoice signing keys; or
- buffer data-encryption keys (DEKs), because the Sidecar owns and encrypts its
  metering buffer.

When the Sidecar is unavailable, the Runtime process remains available and local
optimization functions continue. Calls that require a protected operation are
evaluated independently against the cached policy's expiry behavior; a failure in
one policy class does not create a process-wide fail-closed state. Metering export
waits in the Sidecar-owned durable buffer rather than moving Platform credentials
or buffer DEKs into the Runtime.

After Sidecar unavailability exceeds a configurable timeout, the Runtime emits an
`EvidenceGapOpenedV1` event. The event marks the start of an evidence discontinuity
for later reconciliation and audit; it does not itself override policy expiry
behavior. Recovery must close or reconcile the gap through the evidence protocol,
resume from an accepted sequence state, and use idempotency keys when replaying
pending operations.

## Consequences
The OSS/proprietary boundary is narrow and auditable: all Platform communication,
credential renewal, policy synchronization, and metering export pass through one
component. Platform private keys and Sidecar buffer DEKs remain outside the
Runtime's compromise domain. Transport-specific authentication can use the
strongest primitive available for each deployment while envelope semantics and
tenant isolation remain consistent.

Local optimization remains usable during Sidecar or Platform outages. Policy-
protected operations degrade according to explicit, signed risk decisions rather
than a global availability switch, and `EvidenceGapOpenedV1` makes extended
outages visible to audit and reconciliation systems.

The design adds implementation and operational complexity. Four transports require
conformance testing against identical envelope semantics. Deployments must manage
mTLS identity and renewal where OS-authenticated IPC is unavailable. Sequence
state, clock handling, cached policy validity, alerts, grace periods, and evidence-
gap reconciliation must be durable and observable. Incorrect policy classification
can either cause avoidable outages or permit operations longer than intended, so
policy authorship and review become security-sensitive activities.

## Alternatives Considered
**Direct Runtime-to-Platform connection.** Rejected because it would distribute
Platform connectivity, identity, retry, buffering, and policy-enforcement logic
into the OSS Runtime. It would widen the trust boundary and make offline metering
and credential custody harder to isolate and audit.

**A single transport, such as mTLS for every deployment.** Rejected because
deployment environments have materially different trust primitives. Mandatory
mTLS on a same-process-host installation adds certificate lifecycle cost while
discarding authenticated OS credentials; local IPC alone cannot securely support
separate pods or remote Sidecars. The abstraction preserves one protocol across
the required deployment diversity.

**Runtime custody of private signing or buffer-encryption keys.** Rejected because
a compromised Runtime would then be able to forge Platform policy or evidence,
impersonate settlement/invoice authority, or decrypt buffered metering data. The
Runtime requires verification capability and short-lived session authority, not
Platform signing authority or Sidecar storage keys.

**One global failure policy.** Rejected because always failing closed would make a
Sidecar outage disable unrelated local functionality, while always failing open
would make critical controls ineffective at expiry. Per-policy criticality and
expiry behavior express the required risk/availability trade-off explicitly.

## References
- Platform Architecture Rebuild v5 (plan)
- RFC 8446, The Transport Layer Security (TLS) Protocol Version 1.3
- NIST SP 800-57 Part 1 Rev. 5, Recommendation for Key Management
