# ADR-010: OSS and Proprietary Contribution Boundary

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx needs an open runtime that users can inspect, deploy locally, and integrate
without trusting opaque instrumentation. It also needs a sustainable proprietary
platform whose attribution, verification, orchestration, settlement, and governance
capabilities constitute the commercial product and its core intellectual property.

An ambiguous open-core boundary would make contribution review unpredictable. It
could move commercial methodology into the Apache-2.0 repository, or conversely
hide runtime observation behind proprietary code and weaken user trust. The boundary
must therefore be based on responsibility and data authority rather than deployment
location or whether a component communicates with the platform.

Three distinctions are especially important. A runtime estimate is not a verified
financial result; enforcing a received policy is not authoring governance; and
executing a received experiment assignment is not orchestrating an experiment.
Those distinctions must remain visible in types, modules, APIs, and contribution
review.

## Decision
The architecture adopts this boundary:

> Open instrumentation. Proprietary attribution, orchestration, and settlement.

The OSS `lean-ctx` project is licensed under Apache-2.0. It executes requests,
optimizes local context, observes runtime facts, and enforces authenticated
instructions. The proprietary `lean-ctx-platform` normalizes observations,
preserves authoritative evidence, attributes outcomes, verifies savings,
orchestrates experiments, authors governance, and performs financial settlement.

The OSS scope consists of:

- `lean-ctx-protocol`, containing wire-contract data types only, including
  `MoneyV1`, `UsageEventV1`, `QualityEventV1`, `SavingsObservationV1`,
  `PolicyBundleV1`, `PolicyDecisionV1`, `ExperimentAssignmentV1`, and evidence
  envelope types; it contains no attribution, pricing, settlement, or policy-
  authoring behavior;
- the runtime proxy, caching, compression, local optimization, request execution,
  and offline buffering;
- instrumentation that emits `UsageEventV1`, `QualityEventV1`, and
  `SavingsObservationV1` from locally observable facts;
- deterministic execution of proprietary experiment assignments, including local
  validation and safety enforcement, but not experiment definition, assignment,
  cohort strategy, evaluation, or promotion;
- enforcement of signed policy bundles, but not policy authoring, signing,
  distribution, organizational approval, or fleet governance; and
- a local savings ledger containing explicitly labeled estimates, but never
  verified or billable savings.

The proprietary `lean-ctx-platform` scope consists of:

- **Intelligence:** normalization, enrichment, correlation, and preservation of
  evidence received from runtimes and external sources;
- **Optimize:** baseline construction, causal attribution, quality gates, and the
  production of `VerifiedSavingsV1`;
- **Settlement:** fee calculation, statements, invoice-line rounding, invoices,
  and Stripe integration;
- **Govern:** policy authoring, organizational approval, signing, distribution,
  budget control, audit, and revocation;
- **Connect:** managed connectors and the credential vault;
- **Organizations:** tenant isolation, identity, membership, entitlements, and
  billing accounts; and
- **Console:** the hosted administrative and analytical user interface.

`SavingsObservationV1` and `VerifiedSavingsV1` are deliberately different types
with different authorities. The OSS runtime may emit an observation such as:

```rust
SavingsObservationV1 {
    // Runtime-observed inputs and an explicitly unverified estimate.
    // This value is neither billable nor an attribution decision.
}
```

Only Optimize may produce a verified result after applying proprietary baselines,
attribution methodology, evidence checks, and quality gates:

```rust
VerifiedSavingsV1 {
    // Platform-attributed result backed by preserved evidence and methodology.
    // This is eligible for downstream settlement subject to contract terms.
}
```

Renaming, wrapping, signing, or persisting a `SavingsObservationV1` does not turn
it into `VerifiedSavingsV1`. OSS APIs, logs, dashboards, and local ledgers must use
terms such as `observed`, `estimated`, or `unverified`; they must not describe a
runtime-produced value as verified, attributable, billable, or settled.

Entitlements and governance compose differently and must not be conflated.
Entitlements are an extension union: a valid Sidecar entitlement may enable
additional runtime capabilities beyond the OSS default set. Governance is a
restriction intersection: every executed action must remain within both the
enabled capability set and all applicable policy constraints.

```text
effective_capabilities = oss_capabilities ∪ sidecar_entitlements
allowed_actions = effective_capabilities ∩ governance_policy
```

A policy can restrict or deny an entitled capability; it cannot grant a capability
that is absent from the effective capability set. The OSS runtime performs the
final local enforcement needed to fail closed, while the proprietary platform
authors, signs, distributes, and audits the governing policy.

Experiment ownership follows the same decision-versus-execution boundary. The
proprietary platform defines experiments, selects populations and arms, issues
signed `ExperimentAssignmentV1` values, evaluates results, and decides promotion
or termination. OSS validates and deterministically executes the assignment under
its declared provider, model, cost, data-classification, side-effect, expiry, and
kill-switch bounds. OSS does not infer a new assignment or evaluate experiment
success.

Contribution review applies these rules:

- contributions are welcome in OSS for runtime execution, protocol data types,
  caching, compression, local optimization, instrumentation, deterministic
  assignment execution, policy enforcement, and local estimated ledgers;
- changes that implement or disclose attribution logic, baseline methodology,
  quality-gate methodology, commercial pricing, fee calculation, settlement,
  policy authoring or distribution, managed credential custody, tenant billing,
  or proprietary orchestration belong in `lean-ctx-platform`; and
- a contribution spanning both responsibilities must be split at a versioned wire
  contract. The OSS portion may define and emit neutral input data or enforce a
  signed instruction; the proprietary portion makes the commercial or governance
  decision.

Protocol additions are not automatically acceptable merely because they are data
types. They must support interoperable observation or execution without encoding
proprietary methodology. In particular, `VerifiedSavingsV1`, pricing schedules,
attribution weights, baseline-selection rules, and settlement calculations remain
outside `lean-ctx-protocol`.

## Consequences
The open runtime remains independently deployable, inspectable, and auditable.
Users can verify what telemetry is collected and how local enforcement behaves,
which supports adoption and trust. Integrators receive stable Apache-2.0 wire
contracts and can contribute improvements to the runtime and instrumentation.

The proprietary platform retains the differentiated methodology and operational
systems required for a viable business: cross-source evidence, attribution,
verification, orchestration, governance, and financial settlement. Distinct
`SavingsObservationV1` and `VerifiedSavingsV1` types prevent an estimate from
silently entering a billing path.

The boundary adds integration and review overhead. Features that cross it require
explicit contracts, compatible versioning, and separate changes in two
repositories. Some behavior may appear duplicated because OSS must enforce local
safety while the platform manages fleet-wide governance. Offline operation is
necessarily limited: the runtime can continue supported local functions and record
estimates, but it cannot independently verify savings, issue new entitlements,
or settle fees.

The proprietary methodology is less directly inspectable than OSS instrumentation.
Trust therefore depends on evidence-preserving interfaces, auditable outputs,
contractual methodology, and clear labeling of observation versus verification.
Contributors may also need maintainers to redirect or split proposals whose
apparently generic implementation would expose proprietary decision logic.

## Alternatives Considered
### Fully open-source platform
Rejected because attribution methodology, savings verification, experiment
orchestration, governance, pricing, and settlement are the product's core
intellectual property and commercial differentiation. Opening the complete stack
would leave no viable boundary for the managed business model.

### Fully proprietary platform
Rejected because users could not independently inspect instrumentation, local data
handling, optimization, or enforcement. That would impede community adoption,
third-party integrations, self-hosted runtime use, and trust in the observations
that feed verification and billing.

### Open core with a case-by-case or deployment-based boundary
Rejected because an unclear rule invites accidental intellectual-property leakage,
inconsistent contribution decisions, and architectural coupling. Deployment is
not a reliable classifier: an OSS runtime may enforce a proprietary policy, while
a hosted service may operate entirely on open protocol data. The adopted boundary
instead assigns observation and execution to OSS, and decision authority,
attribution, orchestration, governance authorship, and settlement to the
proprietary platform.

## References
- Platform Architecture Rebuild v5 (plan)
- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- ADR-001: Protocol Scope and Versioning
- ADR-005: Experiment Assignment and Shadow Safety
