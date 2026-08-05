# ADR-005: Experiment Assignment and Shadow Safety

## Status
Accepted

## Date
2026-08-05

## Context
lean-ctx has an open-source (OSS) runtime that executes context-optimization requests and a proprietary platform that owns product policy, experiment configuration, and experiment analysis. Experiment behavior must preserve this boundary: **proprietary decides; OSS executes**.

Allowing the OSS runtime to define experiments, select cohorts from locally configured policy, or evaluate statistical results would duplicate proprietary control-plane responsibilities and make behavior depend on runtime-local state. Conversely, accepting arbitrary execution instructions would expose the runtime to malicious or stale assignments, unauthorized providers or models, uncontrolled incremental spend, and unsafe side effects.

Shadow experiments require additional safeguards because they duplicate a production request. A shadow execution may expose data to another provider or model, consume incremental budget, or repeat a write unless the assignment explicitly constrains those dimensions. Its output must never affect the caller-visible response.

The runtime therefore needs a signed, bounded, and deterministic instruction that completely describes what it may execute, without transferring experiment definition or evaluation into OSS.

## Decision
The proprietary platform defines, configures, assigns, and evaluates experiments. The OSS runtime only validates and executes assignments issued by the proprietary Sidecar and reports execution outcomes.

The normative rule is:

> OSS executes proprietarily-defined and signed assignment rules deterministically. OSS does not configure, define, or evaluate experiments.

The Sidecar sends an `ExperimentAssignmentV1` containing the selected arm and all execution safety bounds:

```rust
struct ExperimentAssignmentV1 {
    experiment_id: String,
    subject_id: String,
    arm: ExperimentArm, // Control | Optimized | Shadow
    configuration_ref: String,
    expires_at: DateTime,
    max_incremental_cost: MoneyV1,
    allowed_providers: Vec<ProviderId>,
    allowed_models: Vec<ModelId>,
    data_classification: DataClass, // Public | Internal | Confidential | Restricted
    side_effect_policy: SideEffectPolicy, // NoSideEffects | ReadOnly | AllowWrites
    kill_switch: KillSwitchRef,
    signature: Ed25519Signature,
}
```

The signature covers every field other than `signature` using the canonical `ExperimentAssignmentV1` serialization. The runtime trusts only configured Sidecar Ed25519 public keys. It rejects the assignment before experiment execution when the signature is invalid, the assignment is expired, or the referenced kill switch is active. It also rejects any provider or model outside `allowed_providers` or `allowed_models` and any execution whose projected incremental cost would exceed `max_incremental_cost`.

After validation, the runtime executes exactly one assigned behavior:

- `Control`: bypass context optimization and execute the baseline request path.
- `Optimized`: execute the normal optimized request path.
- `Shadow`: execute the caller-visible request normally and duplicate it to the assigned shadow path; discard the shadow response unconditionally.

For `Shadow`, all of the following are mandatory:

- `data_classification` must permit the request data to flow through the selected shadow provider and model under the runtime's data-handling policy.
- `side_effect_policy` is enforced independently on the shadow path. `NoSideEffects` forbids external side effects, `ReadOnly` permits only operations classified as reads, and `AllowWrites` permits writes only when the assignment explicitly selects it.
- `max_incremental_cost` is a hard upper bound for cost attributable to the duplicate execution. The shadow path must not start, or must be terminated when safely possible, if the bound cannot be honored.
- `kill_switch` is checked before starting the shadow path and remains the authority for immediate experiment termination. No new shadow work may start while it is active.
- The shadow response is never returned to the caller, substituted for the primary response, or used to mutate caller-visible request state.

The OSS implementation module is named `experiment_executor.rs`, replacing `experiment_runner.rs` to reflect its limited responsibility. It receives an assignment, performs validation and bounded execution, and reports an outcome to the Sidecar. It contains no experiment configuration surface and no statistical evaluation logic.

The former local cohort helper `is_holdout()` is replaced with:

```rust
fn execute_bucketing_rule(
    seed: &str,
    subject: &str,
    holdout_pct: Percentage,
) -> bool;
```

`execute_bucketing_rule` is only a deterministic fallback executor for a bucketing rule supplied by the Sidecar. Its inputs, including `seed` and `holdout_pct`, are assignment data rather than runtime-authored experiment policy. Equal inputs must produce equal results across supported runtime instances and restarts. The helper does not choose an experiment, configure its population, or interpret its outcome.

Outcome reporting identifies the experiment, subject, assigned arm, assignment/configuration reference, validation or execution status, incurred incremental cost as `MoneyV1`, and safety-policy rejections. Reporting is telemetry for proprietary evaluation; the OSS runtime does not calculate significance, compare arms, promote configurations, or change future assignment policy.

Fail-closed behavior applies to assignment validation and safety constraints. A rejected or unavailable experiment assignment does not authorize optimized or shadow execution; the request follows the non-experiment baseline path when that path remains safe and valid, and the rejection is reported.

## Consequences
Positive consequences:

- Experiment ownership is unambiguous: proprietary systems make policy decisions, while OSS behavior is a deterministic execution mechanism.
- Ed25519 signatures prevent unauthorized mutation or fabrication of assignments within the runtime trust model.
- Expiry and the kill switch bound the lifetime of an assignment and permit rapid termination without a runtime deployment.
- Provider, model, data-classification, side-effect, and `MoneyV1` cost constraints make shadow duplication explicitly bounded and auditable.
- Discarding shadow responses prevents experimental output from affecting production callers.
- Centralized outcome evaluation avoids divergent statistics implementations and local experiment state across OSS deployments.

Negative consequences:

- Experiments depend on Sidecar availability for new signed assignments and kill-switch state.
- Signature verification, canonical serialization, expiry handling, and trusted-key rotation add runtime complexity.
- Fail-closed validation can reduce experiment coverage when assignments are stale, malformed, or cannot be verified.
- Enforcing a hard incremental-cost cap may require conservative estimation and may skip potentially useful shadow executions.
- Shadow execution still consumes latency-independent compute and provider budget even though its response is discarded.
- `AllowWrites` remains intrinsically risky and requires explicit proprietary authorization plus runtime enforcement; most shadow assignments should use `NoSideEffects` or `ReadOnly`.

## Alternatives Considered
### OSS defines or decides experiments
Rejected because it violates the proprietary-control-plane boundary, introduces runtime-local configuration and cohort policy, and risks inconsistent assignment and evaluation across deployments. OSS may execute a supplied deterministic bucketing rule, but it may not author that rule or decide when it applies.

### Execute assignments without a kill switch
Rejected because expiry alone cannot terminate an active or costly experiment quickly. A malformed or unexpectedly expensive shadow assignment could continue duplicating requests until expiration and create unbounded aggregate cost or exposure.

### Accept unsigned assignments
Rejected because an attacker or misconfigured intermediary could select unsafe arms, extend expiry, authorize providers or models, raise `max_incremental_cost`, relax `data_classification`, or enable writes. A signature binds the complete instruction to a trusted Sidecar issuer.

### Return or conditionally use shadow responses
Rejected because shadow traffic must be observational. Allowing its output to affect caller-visible behavior would turn the shadow arm into an uncontrolled serving path and invalidate its safety isolation.

### Evaluate experiment statistics in the OSS runtime
Rejected because evaluation requires cross-subject aggregation, experiment-specific metrics, and promotion policy owned by the proprietary platform. Runtime evaluation would duplicate logic and could cause local observations to alter execution policy.

## References
- Platform Architecture Rebuild v5 (plan)
- [RFC 8032: Edwards-Curve Digital Signature Algorithm (EdDSA)](https://www.rfc-editor.org/rfc/rfc8032)

