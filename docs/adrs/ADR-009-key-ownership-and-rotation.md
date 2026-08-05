# ADR-009: Key Ownership and Rotation

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx crosses three security domains: the open-source Runtime, the Enterprise
Sidecar, and the proprietary Platform. These components need cryptographic keys
for distinct purposes: authenticating transport sessions, verifying signed
policies and events, encrypting durable Sidecar buffers, wrapping data-encryption
keys, and producing authoritative Platform signatures.

Key custody must follow least privilege. The Runtime processes untrusted tool and
workload input and therefore has the broadest exposure. Compromise of a Runtime
must not let an attacker forge policies, events, invoices, or settlements; decrypt
the Sidecar's durable buffer; unwrap stored encryption keys; or obtain customer
keys. The Runtime needs only enough authority to authenticate a short-lived
session and verify Platform signatures.

Rotation must not require downtime or create verification gaps. Distributed
deployments cannot replace every cached key, credential, encrypted record, and
signer atomically. A rotation protocol must therefore distinguish key publication,
activation, retirement, and revocation, and it must allow old and new keys to
overlap for a bounded period.

The architecture signs typed protocol artifacts. This includes policy envelopes
and evidence or metering events whose payloads may contain concrete versioned
types such as `SavingsObservationV1` and `MoneyV1`. A signature authenticates the
canonical serialized envelope, including its type/version, tenant binding, key
identifier, and validity metadata; it does not change the semantics of those
payload types.

## Decision
Keys are classified by purpose and assigned to the narrowest component that needs
their authority:

| Class | Owner | Storage | Rotation | Example |
|---|---|---|---|---|
| Public Verification | Runtime | Memory, received from Sidecar | On policy refresh | Policy signature verification |
| Session Credentials | Runtime | Ephemeral memory | Every mTLS renewal | Transport authentication |
| DEK (Data Encryption Key) | Sidecar | Encrypted on disk | On spool rotation | Durable buffer encryption |
| KEK (Key Encryption Key) | Platform KMS | HSM/KMS | Quarterly | Wrapping Sidecar DEKs |
| Signing Keys | Platform | KMS/HSM | On demand | Event, policy, and invoice signing |

The Runtime may receive only:

- public verification keys and their signed metadata;
- short-lived session credentials required for mTLS; and
- signed policies and other signed control-plane artifacts.

The Runtime never receives or persists:

- private signing keys;
- long-term root keys or certificate-authority private keys;
- customer-owned keys;
- DEKs, including Sidecar spool DEKs; or
- plaintext KEKs or an API capability that permits arbitrary KMS decrypt/sign
  operations.

All event and policy signatures use Ed25519. A signed envelope identifies its key
with a stable, non-secret key identifier (`key_id`) and declares the signature
algorithm. Verification is performed over a deterministic canonical byte
representation that includes the envelope type and version, tenant identifier,
payload, issuance time, and validity interval. For example, a signed
`SavingsObservationV1` envelope containing a `MoneyV1` value is verified as the
complete envelope, not by signing an ambiguous JSON fragment or the monetary
amount alone.

Private signing operations occur behind the Platform KMS/HSM boundary. Application
services request a purpose-scoped signing operation and receive a signature; they
do not export private key material. Policy, event, and invoice signing keys use
separate key identities and authorization policies. Possession of authority to
sign one artifact class does not grant authority to sign another.

`platform-kernel` defines the deployment-independent KMS contract. Deployments
provide implementations for their selected KMS or HSM. The contract exposes
purpose-scoped operations rather than raw key bytes, conceptually:

```rust
pub trait KeyManagementService {
    async fn sign_ed25519(
        &self,
        key: SigningKeyRef,
        purpose: SigningPurpose,
        message: &[u8],
    ) -> Result<Signature, KeyManagementError>;

    async fn wrap_dek(
        &self,
        kek: KekRef,
        dek: SecretDek,
        context: WrappingContext,
    ) -> Result<WrappedDek, KeyManagementError>;

    async fn unwrap_dek(
        &self,
        kek: KekRef,
        wrapped: &WrappedDek,
        context: &WrappingContext,
    ) -> Result<SecretDek, KeyManagementError>;
}
```

`SigningKeyRef` and `KekRef` are opaque references. Implementations must prevent
private material from being returned through the trait. Authorization binds each
operation to its purpose, tenant or deployment context where applicable, and an
auditable service identity.

The Sidecar owns durable buffering. It generates or obtains a DEK through its own
Vault/KMS integration, encrypts buffered records locally, and persists only
ciphertext plus the wrapped DEK and non-secret key metadata. The KEK remains in
the Platform KMS/HSM. Each spool rotation creates a new DEK; existing spool
segments retain the wrapped DEK needed for decryption until their retention period
ends. DEKs are never shared with the Runtime because the Runtime has no persistent
buffer.

Rotation is automated and uses a zero-downtime overlap protocol:

1. Generate the successor key or credential in its owning security domain.
2. Publish signed public metadata for the successor, including `key_id`, purpose,
   algorithm, `not_before`, and `not_after`, before activating it.
3. Refresh verifiers and confirm that both predecessor and successor public keys
   are available.
4. Activate the successor for new signatures, sessions, or spool segments.
5. Continue accepting artifacts produced by the predecessor only during the
   bounded overlap window and only within the artifact's own validity interval.
6. Stop issuing with the predecessor, retire it after the maximum artifact,
   cache, retry, and clock-skew window has elapsed, and remove it from verifier
   sets on a subsequent refresh.

The overlap duration is configuration owned by the relevant key policy. It must be
long enough for policy refresh, in-flight requests, retries, mTLS renewal, and
bounded clock skew, but it is not indefinite. Verification requires a recognized
`key_id`, the expected Ed25519 algorithm and signing purpose, a valid key interval,
and a valid artifact interval. Unknown, retired, mismatched-purpose, or expired
keys fail verification.

Routine signing-key rotation is on demand and may be scheduled by Platform policy.
KEKs rotate at least quarterly. KEK rotation rewraps active DEKs without exposing
plaintext key material outside the KMS/Sidecar boundary; bulk payload data is not
re-encrypted solely because its KEK changed. Session credentials rotate at every
mTLS renewal. Public verification sets refresh with signed policy distribution.

Emergency revocation is distinct from normal overlap. A compromised key is marked
revoked in signed key metadata, issuance stops immediately, and verifiers reject
it regardless of its previous `not_after`. This may invalidate affected artifacts
or sessions and therefore can reduce availability; security takes precedence over
the zero-downtime goal during confirmed compromise.

The architecture quality gate includes **Secret/Key Rotation Tests**. Automated
tests must prove at minimum:

- the Runtime cannot obtain signing keys, root keys, customer keys, DEKs, or
  unrestricted KMS capabilities;
- old and new Ed25519 keys both verify during overlap, new issuance uses only the
  successor, and the predecessor fails after retirement;
- unknown `key_id`, algorithm substitution, purpose confusion, invalid validity
  intervals, and revoked keys are rejected;
- mTLS credentials renew without interrupting established valid traffic and old
  credentials cease to authenticate after their allowed interval;
- spool rotation creates a new DEK, old segments remain decryptable through their
  wrapped DEKs, and the Runtime never participates in decryption; and
- quarterly KEK rotation rewraps active DEKs and preserves buffer availability
  without exporting KEK or DEK plaintext.

## Consequences
Compromising the Runtime yields verification material and short-lived transport
credentials, but not authority to forge Platform artifacts or decrypt durable
Sidecar data. Purpose-separated signing keys constrain the impact of a compromised
Platform service identity. Sidecar-owned DEKs align encryption authority with the
only component that maintains a persistent buffer.

Automated overlap permits policies, events, sessions, and spool segments to remain
available during routine rotation. Opaque KMS references and a shared
`platform-kernel` contract make custody rules consistent while allowing different
deployment-specific KMS/HSM providers.

The design adds operational state and testing requirements. Systems must publish
and cache multi-key verification sets, retain old wrapped DEKs for live spool
segments, calculate safe overlap windows, handle clock skew, audit KMS operations,
and distinguish retirement from emergency revocation. Retaining predecessor public
keys during overlap temporarily expands the set of accepted verification keys,
and emergency revocation can intentionally interrupt availability.

Ed25519 standardizes event and policy signing but requires deterministic envelope
serialization and explicit key-purpose separation. Existing integrations using a
different signature scheme must be adapted at the boundary rather than introducing
per-artifact algorithm negotiation into the Runtime.

## Alternatives Considered
**Runtime holds private signing keys.** Rejected because a compromised Runtime
would gain authority to forge policies, evidence events, invoices, or settlements.
The Runtime needs verification capability, not signing authority.

**A DEK shared by Runtime and Sidecar.** Rejected because the Runtime has no
persistent buffer and therefore no legitimate need to decrypt Sidecar spool data.
Sharing the DEK would widen the compromise domain without enabling a required
operation.

**Manual key rotation.** Rejected because coordinated publication, activation,
overlap, retirement, rewrapping, and credential renewal are repetitive and
time-sensitive. Manual execution increases human error, outage, stale-key, and
compliance risk and cannot provide a continuously enforced rotation schedule.

**Immediate replacement without overlap.** Rejected because verifiers, in-flight
requests, cached policies, retry queues, and distributed sessions cannot update
atomically. Immediate retirement would create avoidable verification failures and
downtime during normal rotation.

**One shared Platform signing key for every artifact class.** Rejected because it
would allow compromise or misuse of one signing service to forge unrelated artifact
classes. Separate purpose-bound keys reduce blast radius and support narrower KMS
authorization and audit policy.

## References
- Platform Architecture Rebuild v5 (plan)
- RFC 8032, Edwards-Curve Digital Signature Algorithm (EdDSA)
- NIST SP 800-57 Part 1 Rev. 5, Recommendation for Key Management
- NIST SP 800-38F, Recommendation for Block Cipher Modes of Operation: Methods for Key Wrapping
