# Agent Identities — Registered, Attested, Revocable

GitLab: `#433` (H3 Epic D) · Module: `core/agent_registry.rs` · CLI: `lean-ctx agent`

AI agents stop being anonymous processes with a role config and become
**registered identities**: unique, owned by a human, lifecycle-managed,
auditable and revocable. This is the engine-side foundation for workforce
governance — an org that runs 50 agents must be able to answer *who runs,
who owns, who switched off which agent, and when*.

## Model

| Field | Meaning |
|---|---|
| `agent_id` | Stable identity (key); `[A-Za-z0-9_-]` |
| `role` | Permission profile (`roles/*.toml` / built-ins) — *what it may do* |
| `owner` | **Mandatory** human accountable — *who answers for it* |
| `status` | `active` → `suspended` ⇄ `active` → `decommissioned` (final) |
| `public_key` | Ed25519 key bound to the identity (signs audit entries) |
| `attestation` | Binary + role-config SHA-256 at registration/heartbeat |
| `last_heartbeat` | Liveness timestamp |

Identity (who) is deliberately separate from role (what): roles stay
reusable profiles; accountability attaches to the identity.

## Lifecycle

```
lean-ctx agent register --id ci-reviewer-1 --role reviewer --owner alice@org
lean-ctx agent heartbeat ci-reviewer-1          # liveness + drift check (exit 3 on drift)
lean-ctx agent suspend ci-reviewer-1 --reason "incident IR-42"
lean-ctx agent resume ci-reviewer-1
lean-ctx agent decommission ci-reviewer-1       # final; writes the audit-closing entry
lean-ctx agent check ci-reviewer-1              # enforce-path check (exit 1 = deny)
```

Every transition writes a tamper-evident audit entry (event types
`agent_registered`, `agent_suspended`, `agent_resumed`,
`agent_decommissioned` — OCP Part 4, included in evidence bundles).
Decommissioned identities are never deleted and never reactivated: the
record is part of the audit history.

## Owner offboarding (the orphaned-agent problem)

Orphaned agents — running identities whose human owner left — are the
security hole of the agent era. The registry closes it mechanically:

```
lean-ctx agent offboard-owner alice@org --reason "left the company"
```

suspends **every active agent owned by alice@org** in one transaction and
audits each suspension. Wire this to your IdP:

* **SCIM** (ENT-2): on `active=false` for a user, call
  `agent_registry::suspend_agents_for_owner(user, "SCIM deactivated")`
  (HTTP path: team-server SCIM handler) or run the CLI from your
  deprovisioning pipeline.
* **Manual**: part of the leaver checklist.

Policy choice (suspend vs. transfer) stays with you: suspended agents can
be `resume`d after `register`-ing a new owner via decommission + re-register.

## Attestation — honest threat model

`register` and `heartbeat` hash the running binary and the active role
file. A drifted hash (exit code 3) tells you *something changed* —
upgrade, config edit, or tampering. **This is drift detection, not proof
of integrity**: an attacker with full host control can fake hashes. What
it does give you:

* unnoticed config/binary changes surface in regular heartbeats,
* the attestation history is part of the audit chain (tamper-evident
  after recording),
* combined with evidence bundles, an auditor can see *when* the fleet
  drifted.

It does NOT replace host hardening, code signing or supply-chain controls.

## Workload IAM (SPIFFE)

Every record maps to a SPIFFE-compatible workload identity:

```
spiffe://<trust-domain>/agent/<role>/<agent_id>
lean-ctx agent show ci-reviewer-1 --trust-domain org.example
  → spiffe://org.example/agent/reviewer/ci-reviewer-1
```

Kubernetes reference setup (SPIRE): register the node + workload with the
same path scheme so the agent's K8s service account maps 1:1 to its
LeanCTX identity:

```
spire-server entry create \
  -parentID  spiffe://org.example/ns/agents/sa/leanctx \
  -spiffeID  spiffe://org.example/agent/reviewer/ci-reviewer-1 \
  -selector  k8s:ns:agents -selector k8s:sa:ci-reviewer-1
```

The OIDC client-credentials path (agent visible as a service account in
Entra/Okta) builds on the team-server token plane and is tracked as the
hosted half of #433 — engine-side prerequisites (stable identity, status
check, owner binding) are what this module provides.

## Enforce mode

`agent_registry::check(agent_id)` is the single decision point: not
registered ⇒ deny; suspended/decommissioned ⇒ deny; active ⇒ allow.
Call paths (team-server middleware, A2A handlers) consult it in enforce
mode and only log in monitor mode — start in monitor, switch to enforce
once your fleet is registered.
