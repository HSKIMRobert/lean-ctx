# ADR-012: Entitlements vs. Governance

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx must answer two superficially similar but semantically opposite questions.
Feature availability asks what the installed Runtime is capable of doing. Operation
permission asks whether a specific invocation is allowed under security,
organizational, and runtime constraints. Treating both questions as one permission
system makes the merge direction ambiguous and can turn a licensing change into a
security bypass or make an enterprise policy remove baseline OSS functionality.

The OSS Runtime has a stable set of local features. A licensed Enterprise Sidecar
may advertise additional features purchased by the tenant, but the Sidecar is an
extension boundary: its absence, an expired license, or a lower plan must not remove
features already available in the OSS build.

Governance has the opposite requirement. Enterprise customers must be able to
restrict operations that the Runtime can technically perform, including blocking
specific models, denying network egress, blocking tools, limiting budgets, and
enforcing data-classification rules. No organization policy may weaken hardcoded
security requirements, and no transient runtime condition may be ignored merely
because another policy layer permits an operation.

The distinction is therefore between capability and authority:

```text
Feature availability: what CAN the Runtime do?
Operation permission: what MAY this invocation do now?
```

## Decision
Feature availability and operation permission are separate decision paths with
separate types, APIs, ownership, tests, and merge operators. Code must not expose a
generic `Permission`, `merge_permissions`, or shared numeric precedence helper for
both paths.

Feature availability is additive:

```text
AvailableFeatures = LocalOSSFeatures ∪ LicensedFeatures
```

`core/billing/plans.rs` owns entitlement resolution. The local feature set is the
floor and Sidecar-provided licensed entitlements may only extend it. For ordered
plan or capability levels this is equivalent to `max(local, sidecar)`; for feature
sets it is set union. Missing, invalid, expired, or unreachable Sidecar entitlement
input contributes the empty licensed set, leaving all local OSS features intact.

```rust
struct EntitlementSet {
    features: BTreeSet<FeatureId>,
}

fn available_features(
    local: &EntitlementSet,
    licensed: Option<&EntitlementSet>,
) -> EntitlementSet {
    local.union(licensed.unwrap_or(&EntitlementSet::empty()))
}
```

Entitlements answer only whether a capability is available. They do not authorize
a particular model call, tool execution, data transfer, or spend. Entitlement
claims received through the Sidecar must still pass the trust, tenant-binding,
signature, validity, and replay controls defined for Sidecar communication.

Operation permission is restrictive:

```text
AllowedOperation = LocalSecurityFloor
                AND OrganizationPolicy
                AND RuntimeSafetyChecks
```

`core/policy/` owns governance evaluation. The effective decision is the
intersection of every applicable constraint. For ordered limits this is equivalent
to `min(local_floor, org_policy)`; for allow-lists it is set intersection; for
deny-lists it is deny-set union; and for boolean predicates every predicate must
pass. Policy enforcement must never use `max()`.

```rust
struct OperationRequest {
    tenant_id: TenantId,
    model: ModelId,
    tool: Option<ToolId>,
    egress: EgressDestination,
    data_classification: DataClassification,
    estimated_cost: MoneyV1,
}

fn allowed_operation(
    request: &OperationRequest,
    local_floor: &LocalSecurityFloor,
    org_policy: &OrganizationPolicy,
    safety: &RuntimeSafetyChecks,
) -> PolicyDecision {
    local_floor.evaluate(request)
        .and(org_policy.evaluate(request))
        .and(safety.evaluate(request))
}
```

`PolicyDecision::Allow` is returned only when all three layers allow the request.
Each denial records the responsible layer and a stable reason code for auditability;
a denial from one layer cannot be overridden by an allow from another.

The layers have distinct responsibilities:

- `LocalSecurityFloor` contains non-overridable safety restrictions shipped with
  the Runtime. Neither a license nor organization configuration can relax it.
- `OrganizationPolicy` contains tenant-scoped restrictions synchronized through
  the Sidecar. It may block `ModelId` or `ToolId` values, deny egress destinations,
  narrow accepted `DataClassification` values, and lower a budget expressed as
  `MoneyV1`.
- `RuntimeSafetyChecks` contains dynamic guards such as rate limits, circuit
  breakers, current budget consumption, dependency health, and emergency stops.

Evaluation order may short-circuit for efficiency, but it must not change the
intersection semantics. Implementations should evaluate cheap local checks first
and retain sufficient denial evidence for audit and diagnosis.

The end-to-end call sequence is:

```text
requested feature
  -> present in AvailableFeatures?       (union; capability gate)
  -> allowed by every governance layer? (intersection; authority gate)
  -> execute
```

Failure of the capability gate reports an unavailable feature. Failure of the
governance gate reports a denied operation. These outcomes use different error
types and telemetry so callers cannot interpret a policy denial as an upsell or an
unlicensed feature as a security-policy failure.

Tests must encode the algebraic invariants. Entitlement tests prove that adding a
licensed feature never removes a local feature and that Sidecar absence preserves
the local set. Governance tests prove that adding a restriction never expands the
allowed operation set, no organization policy can override `LocalSecurityFloor`,
and a failed `RuntimeSafetyChecks` result always denies execution.

## Consequences
The architecture makes licensing behavior monotonic in the additive direction and
security behavior monotonic in the restrictive direction. OSS capabilities remain
available without the Sidecar, while enterprise administrators can constrain any
locally available operation. Local hardcoded safety remains authoritative, and
dynamic runtime protection can stop an otherwise entitled and policy-compliant
operation.

Separate types and error paths make code review, testing, telemetry, and audit
records more precise. A caller can distinguish “not installed or licensed” from
“available but prohibited,” and commercial code cannot accidentally become an
authorization oracle.

The separation introduces duplicate-looking evaluation infrastructure and requires
call sites to perform both gates. Feature identifiers, model identifiers, tool
identifiers, tenant context, and budget values must be mapped consistently across
billing and policy domains. Engineers must understand that `max` is valid only for
ordered entitlements and is forbidden for policy enforcement; incorrect use of a
shared merge helper is a security defect.

Organization policy depends on Sidecar synchronization and the expiry behavior
defined for policy delivery. Entitlement fallback and policy fallback remain
different: Sidecar entitlement loss falls back to local OSS features, while policy
loss follows the applicable fail-closed, fail-open, or grace-period rule and never
silently converts into entitlement behavior.

## Alternatives Considered
**A single permission system.** Rejected because it conflates capability with
authority and leaves no universally correct merge operator. Union is correct for
additive entitlements but unsafe for governance; intersection is correct for
governance but would let licensing state remove OSS features.

**Allow policy to grant features.** Rejected because a forged, misconfigured, or
overly broad policy could enable code paths that were not locally available or
licensed. Policy may only narrow execution of features already present in
`AvailableFeatures`.

**Allow entitlements to restrict features.** Rejected because it violates the
Sidecar extension boundary and would make an outage, license downgrade, or invalid
entitlement response remove OSS functionality. Entitlements from the Sidecar are
additive only.

**Use `max()` for every ordered decision.** Rejected because larger policy limits
or broader allow-lists weaken governance. Policy limits use the most restrictive
applicable value (`min` for upper bounds), and any mandatory denial remains a
denial.

## References
- Platform Architecture Rebuild v5 (plan)
- ADR-004: Sidecar Trust and Failure Modes
