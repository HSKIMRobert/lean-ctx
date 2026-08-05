# ADR-007: Tenant Isolation Model

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx is a multi-tenant platform. Organizations own contracts, billing accounts,
and the data produced by platform services. A tenant boundary must therefore exist
before settlement, invoicing, or entitlement flows are implemented. Retrofitting
that boundary after those flows exist would leave ownership ambiguous and make
cross-tenant access defects difficult to eliminate.

The core safety invariant is:

> A domain operation never reads, writes, updates, or deletes data belonging to a
> tenant other than the tenant in its `TenancyContext`.

This invariant applies to every access path, including point lookups, list queries,
aggregates, joins, background jobs, commands, and repository methods. Filtering
only user-facing list endpoints is insufficient: identifiers may be guessed or
accidentally reused, and asynchronous work has no request boundary from which to
infer tenancy.

Phase 2 settlement with a Stripe invoice depends on an organization, tenant,
contract owner, billing account, and currency. These concepts must be established
before the end-to-end settlement flow. The architecture therefore separates the
work into Phase 2A (organization and tenancy foundation) followed by Phase 2B
(end-to-end settlement flow).

The default model must support many tenants without requiring a schema or database
per tenant. It must also leave room for enterprise customers that may later require
dedicated isolation.

## Decision
The platform will use query-level tenant isolation by default. Every persisted
tenant-owned row carries a non-null `tenant_id`, and every query against
tenant-owned data includes a tenant predicate equivalent to:

```sql
SELECT ...
FROM contract
WHERE tenant_id = $1
  AND contract_id = $2;
```

The predicate is required for reads and mutations. Updates and deletes scope both
the target identifier and the tenant identifier; inserts take `tenant_id` from the
active tenancy context rather than from an untrusted payload.

`platform-kernel` defines and owns the isolation primitives:

```rust
pub struct TenancyContext {
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub isolation_level: IsolationLevel,
}

pub enum IsolationLevel {
    Shared,    // Query-level isolation; the default.
    Dedicated, // Schema-per-tenant; reserved for a future enterprise tier.
}
```

`TenantId` and `OrganizationId` are strongly typed identifiers. They are not
interchangeable with each other or with raw strings. `TenancyContext` is created at
an authenticated platform boundary and propagated explicitly through application
services, domain operations, repository interfaces, transaction boundaries, and
background-job payloads. Domain code must not infer tenancy from ambient process
state or accept an optional tenant.

Repository APIs for tenant-owned aggregates require a tenancy context (or a
`TenantId` derived from it). An unscoped domain repository method is invalid. For
example:

```rust
pub trait ContractRepository {
    async fn find(
        &self,
        tenancy: &TenancyContext,
        contract_id: ContractId,
    ) -> Result<Option<Contract>, RepositoryError>;
}
```

Joins must preserve the same boundary on all tenant-owned tables. A foreign key or
globally unique object identifier does not replace the tenant predicate. Where
practical, database constraints and composite keys should reinforce tenant-consistent
relationships, but application query scoping remains mandatory.

Cross-tenant access is unavailable to ordinary domain operations. Any future
system-level override must be an explicit, separately authorized capability for a
specific administrative or platform operation. It must not be represented by an
optional `tenant_id`, a sentinel tenant, or omission of `TenancyContext`, and it
must be auditable. Such an override is outside the default repository surface.

Phase 2A establishes the prerequisites for this model:

- The Organization aggregate, including `OrganizationId`, name, and billing
  contact.
- `TenantId`, `TenancyContext`, and `IsolationLevel` primitives.
- Service identity that binds a runtime caller to the platform identity used to
  authorize tenancy access.
- A billing account with an explicit currency.
- Contract ownership that records which organization owns each contract.
- Basic entitlement checks evaluated within the same tenancy boundary.

`IsolationLevel::Shared` is the only implemented storage behavior in Phase 2A.
`IsolationLevel::Dedicated` reserves a stable domain representation for a future
schema-per-tenant enterprise implementation; it does not permit callers to choose a
schema or weaken query scoping in the shared model.

Isolation is verified continuously:

- Every integration-test scenario creates and exercises at least two tenants.
- Each tenant-owned repository and end-to-end flow has negative tests proving that
  tenant A cannot read, mutate, delete, aggregate, or otherwise observe tenant B's
  data, including when tenant B's object identifier is supplied directly.
- Tests cover background and service-to-service execution so service identity and
  `TenancyContext` propagation cannot be bypassed outside HTTP request handling.
- Schema compatibility tests enforce forward-only migrations. Migrations must keep
  tenant scoping valid throughout deployment and must not temporarily expose
  tenant-owned rows without a usable, non-null tenant identifier.

## Consequences
The model provides one uniform tenant boundary across domain services and storage
access. Shared tables remain operationally economical, and normal migrations can be
applied once for all tenants. Explicit context propagation makes ownership visible
in APIs and code review, while strong identifier types reduce accidental mixing of
organization and tenant identities. Establishing these primitives in Phase 2A lets
Phase 2B settlement attach contracts, billing accounts, currency, entitlements, and
Stripe invoices to an unambiguous owner.

Every tenant-owned query must be designed and reviewed for correct scoping. Missing
predicates remain a potential application defect, so repository API design,
multi-tenant integration tests, database constraints, and observability are required
defense layers. Indexes will generally need `tenant_id` as a leading or otherwise
appropriate key component, increasing index width and requiring tenant-aware query
planning. Explicit propagation also adds parameters to service and repository APIs
and requires tenancy metadata in durable background work.

The shared model does not provide the physical isolation of a separate schema or
database. Supporting `Dedicated` later will require routing, provisioning,
migration, backup, and operational tooling, while preserving the same domain-facing
`TenancyContext` contract.

## Alternatives Considered
**Schema per tenant as the default.** Rejected because provisioning, connection and
schema routing, migrations, monitoring, backups, and fleet-wide changes become
operationally complex at platform scale. It remains a future enterprise option
represented by `IsolationLevel::Dedicated`.

**Database per tenant.** Rejected because per-tenant infrastructure and connection
management are too expensive for the default tier, and coordinated migrations,
reporting, backup, and recovery across a database fleet would be substantially more
difficult.

**No isolation and a single-tenant product model.** Rejected because the platform
is multi-tenant by design. Organization ownership, contracts, billing, entitlements,
and settlement require a durable tenant boundary from their first implementation.

**Database Row-Level Security (RLS) as the sole mechanism.** Rejected because it
couples isolation semantics to a specific database and session configuration and
does not cover every access pattern, including non-database resources and domain
operations before persistence. RLS may be added as defense in depth, but it cannot
replace explicit `TenancyContext` propagation and tenant-scoped repository queries.

## References
- Platform Architecture Rebuild v5 (plan)
