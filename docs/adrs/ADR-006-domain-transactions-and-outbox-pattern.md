# ADR-006: Domain Transactions and Outbox Pattern

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx domains must communicate without coupling their database transactions or
availability. A domain operation can change owned state and publish a fact needed by
another domain. For example, `optimize` can turn a `SavingsObservationV1` into a
`VerifiedSavingsV1`, whose monetary values use `MoneyV1`, and
`billing-settlement` can use that verified fact for settlement.

Writing domain state and publishing a message as independent operations creates a
dual-write failure: the state may commit without the message, or the message may be
published while the state rolls back. A transaction spanning two domains would avoid
that narrow problem but introduce runtime coupling, additional latency, and difficult
partial-failure recovery.

Design Principle 7 of Platform Architecture Rebuild v5 therefore requires:
"Outbox Pattern statt Cross-Domain Transactions (Ausnahmen nur mit ADR)" (Outbox
Pattern instead of cross-domain transactions; exceptions require a dedicated ADR).

## Decision
Cross-domain communication uses the Outbox Pattern exclusively. No transaction may
span domain ownership boundaries unless a later, dedicated ADR defines and justifies
an exception.

The producing domain atomically commits its owned state, immutable `DomainEvent`, and
`OutboxEntry` in one local database transaction. Each domain owns a separate outbox
table in its own schema; there is no platform-wide shared outbox table.

For example, `optimize` persists a verified result and its publication intent as one
unit:

```sql
BEGIN;
INSERT INTO optimize.verified_savings (...); -- VerifiedSavingsV1
INSERT INTO optimize.domain_events (...);   -- immutable DomainEvent
INSERT INTO optimize.outbox
    (entry_id, event_id, event_type, payload, idempotency_key, sequence, status)
VALUES
    (..., ..., 'VerifiedSavingsV1', ..., ..., ..., 'pending');
COMMIT;
```

The responsibilities are separated as follows:

- The producing domain creates the event and outbox entry inside its local
  transaction.
- `storage-postgres/outbox_store.rs` provides `PostgresOutboxStore`, which only
  persists, reads, and updates outbox entries. It is not a shared consumer and does
  not perform transport delivery.
- An `OutboxDispatcher`, hosted by `connector-worker` or `platform-server`, polls a
  domain's outbox, sends pending entries, and records successful delivery.
- The receiving domain processes the message idempotently and owns its own inbox or
  equivalent deduplication state.

Every `OutboxEntry` has a stable event identity and `idempotency_key`. The receiver
must atomically record that key with its own domain-state change. If the key was
already recorded, the receiver treats the redelivery as successful without applying
the change again.

Entries are sequenced and dispatched in order within each producing domain. A failed
entry blocks later entries from that domain until retry or explicit operational
resolution; ordering is not guaranteed across different domains. Dispatchers may
use locking or leasing to coordinate concurrent workers, but must preserve the
domain sequence.

Delivery is at least once. A dispatcher marks an entry delivered only after the
receiver or transport acknowledges it. A crash after delivery but before that state
update causes redelivery, which is why receiver-side idempotency is mandatory. The
outbox does not claim exactly-once transport semantics.

The representative flow is:

```text
optimize produces VerifiedSavingsV1
  -> commits optimize.verified_savings + DomainEvent + optimize.outbox
  -> OutboxDispatcher polls optimize.outbox in sequence
  -> dispatches the event to billing-settlement
  -> billing-settlement deduplicates by idempotency_key and commits locally
  -> receiver acknowledges successful handling
  -> OutboxDispatcher marks the outbox entry delivered
```

## Consequences
Positive consequences:

- Domain state and publication intent cannot diverge because they commit atomically.
- Producers and consumers remain independently available and deployable.
- Durable pending entries support retries and operational inspection.
- Per-domain tables preserve schema, retention, access-control, and migration
  ownership.
- Store and dispatcher implementations can evolve independently.

Negative consequences:

- Consumers must implement durable idempotency and retain deduplication state for an
  appropriate period.
- Delivery is asynchronous, so cross-domain reads are eventually consistent.
- Duplicate delivery is expected and must be tested.
- Strict per-domain ordering can cause head-of-line blocking when an entry repeatedly
  fails.
- Operations must monitor backlog, retries, poison entries, and dispatcher leases.
- Each domain must migrate and operate its own outbox and inbox/deduplication state.

## Alternatives Considered
- **Distributed transactions (2PC):** Rejected because they couple domain
  availability, add coordination latency, and create difficult coordinator and
  participant partial-failure modes.
- **Direct service-to-service calls:** Rejected because a caller's commit becomes
  dependent on receiver availability and a failed call has no durable publication
  record.
- **Shared message queue without a transactional outbox:** Rejected because publishing
  to the queue and committing domain state remain a non-atomic dual write.
- **One shared outbox table:** Rejected because it violates domain ownership and
  centralizes schema evolution, access control, retention, and failure contention.

## References
- Platform Architecture Rebuild v5 (plan)
- [Transactional Outbox](https://microservices.io/patterns/data/transactional-outbox.html)
- [Idempotent Consumer](https://microservices.io/patterns/communication-style/idempotent-consumer.html)
