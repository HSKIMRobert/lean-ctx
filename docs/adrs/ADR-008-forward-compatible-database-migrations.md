# ADR-008: Forward-Compatible Database Migrations

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx must deploy schema and application changes without stopping writes or
requiring every process to change version atomically. During a rolling deployment,
the database may be accessed concurrently by the previous binary, the new binary,
background workers, and processes restarted from an older release. The schema must
therefore remain compatible with both application versions for the entire rollout.

This requirement applies to all persisted domain data, including representations
of versioned types such as `MoneyV1` and `SavingsObservationV1`. A version suffix on
an application type does not make a destructive database migration safe: binaries
that still read or write the previous representation remain active until deployment
completion is proven.

Database rollback migrations are unsafe in this environment. Once the new binary
has written data using an expanded schema, reversing that schema can discard data
or make it unreadable. The operational recovery path must instead preserve the
database and restore a binary known to work with that schema.

The architecture therefore adopts Design Principle #8: **Forward-only Migrations
(expand-and-contract)**.

## Decision
Every database schema change will be implemented as one or more forward-only
expand-and-contract migrations. Applied migrations are never reversed in
production. Migration identifiers are immutable; correcting an applied migration
requires a new forward migration.

### Expand phase

The expand phase changes the schema additively while preserving the behavior and
data contract required by the previous binary. Permitted operations include:

- adding a nullable column;
- adding a column with a safe database default;
- adding a new table, index, or non-breaking constraint;
- adding a parallel representation for a value whose shape or type must change.

The old binary must be able to ignore every added object. The new binary must
tolerate records created by the old binary, including absent values in newly added
nullable columns. A new required field is introduced as nullable or defaulted,
backfilled separately, and enforced only after compatibility and data completeness
have been established.

For example, replacing a legacy amount representation with the persisted shape
used by `MoneyV1` requires parallel storage rather than an in-place type change:

```sql
-- Expand: additive and compatible with the old binary.
ALTER TABLE savings_observation
    ADD COLUMN amount_minor BIGINT NULL,
    ADD COLUMN amount_currency CHAR(3) NULL;
```

During transition, application code uses an explicit compatibility strategy such
as dual-write plus fallback-read. Existing rows are backfilled with an idempotent,
restartable job. `SavingsObservationV1` must not assume the new columns are populated
until the backfill and writer migration are complete.

### Contract phase

The contract phase removes the legacy representation only after all of the
following are true:

- every production binary and worker is on a version that no longer reads or
  writes the legacy columns or tables;
- rollback to any binary requiring the legacy representation is no longer an
  operational requirement;
- the backfill is complete and verified;
- compatibility telemetry shows no legacy reads or writes;
- backups, replicas, maintenance jobs, exports, and operational tooling have been
  checked for legacy dependencies.

Contract is a separate deployment, not a later statement in the expand migration.
It may add validated constraints such as `NOT NULL` and may drop obsolete columns
or tables. After contract, recovery remains forward-only: a defect is fixed by a
new migration and/or a compatible binary, not by reverting the database schema.

### Prohibited operations during expand

An expand migration must never:

- rename a column or table in place;
- destructively change a column type;
- drop a column or table;
- add a constraint that existing rows or the old binary can violate;
- change defaults or semantics in a way that changes old-binary behavior;
- require the old binary to know about the new schema object.

A rename is modeled as add, copy/backfill, transition reads and writes, then drop in
contract. A type change is modeled with a new column or table and explicit
conversion. This rule applies even when the database supports a syntactically
atomic rename or cast because application compatibility, not DDL atomicity, is the
governing constraint.

### Compatibility verification

Every migration series must include schema compatibility tests for both rollout
directions:

1. Run the previous binary against the expanded schema and verify its supported
   reads and writes.
2. Run the new binary against the pre-expand schema where the release procedure can
   expose that ordering, or prove that deployment ordering prevents it.
3. Run the new binary against the expanded schema with both pre-backfill and
   post-backfill data.
4. Verify that data written by each binary remains readable by the other wherever
   coexistence requires it.

Compatibility fixtures must include versioned domain values such as `MoneyV1` and
`SavingsObservationV1`, null/default states introduced by expand, and legacy rows
created before the migration. Tests must exercise normal services and background
workers, not only direct SQL.

The deployment sequence is schema expand, compatible binary rollout, backfill and
verification, then a separately approved schema contract after fleet convergence.
Deployment automation must prevent contract while any old binary is registered or
eligible for rollback.

### Rollback and completion gates

Application rollback means deploying the previous binary against the already
expanded schema. It never means reversing an applied database migration. Each
expand change must therefore satisfy the completion gate **“Code Rollback ohne DB
Rollback”**: the previous binary is demonstrably functional without a database
rollback.

Before production rollout, the complete forward migration and representative
backfill must also pass **“Forward-Migration auf Produktionskopie”**: execute on a
sanitized production copy at production-like scale and verify correctness,
duration, locking behavior, disk growth, and application compatibility. A
migration that cannot pass both gates is not deployable.

## Consequences
Rolling deployments can keep old and new binaries online against one database, and
application rollback remains available after schema expansion. Forward-only
history avoids ambiguous down migrations and prevents recovery procedures from
silently deleting data written by a newer release. Separate backfills make large
data transformations observable, resumable, and verifiable.

The approach requires temporary schema duplication, compatibility code, dual reads
or writes, and at least two releases for destructive changes. Storage and index
usage can increase until contract. Teams must track fleet convergence and legacy
access before cleanup, and migration testing requires runnable previous binaries
and representative production data. Contract may be delayed indefinitely when
compatibility cannot be proven; this is preferable to unsafe cleanup.

Forward-only migrations do not eliminate operational risk. DDL must still be
designed to avoid long locks and table rewrites, backfills must be bounded and
restartable, and semantic changes require explicit observability. The decision
changes recovery from schema reversal to compatible binary deployment plus a
subsequent corrective forward migration.

## Alternatives Considered
**Traditional rollback migrations.** Rejected because a down migration can lose
data written after expand, may not reconstruct the prior representation, and can
make a zero-downtime rolling deployment incompatible with the restored schema.
Database rollback is therefore not a reliable inverse of production activity.

**Blue/green deployments with separate databases.** Rejected because concurrent
writes require bidirectional synchronization or a coordinated write cutover.
Replication lag, conflict resolution, identity generation, and validation make the
database transition substantially more complex and create additional failure
modes. Blue/green application fleets may still share one compatibly expanded
database.

**Schema per application version.** Rejected because duplicating schemas wastes
storage, fragments migration state, and requires cross-version data movement or
query routing. Long-lived workers and rolling deployments would multiply active
schemas and make consistency and cleanup harder than maintaining one compatible
schema.

## References
- Platform Architecture Rebuild v5 (plan)
