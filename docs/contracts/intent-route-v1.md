# Intent Route v1 (IntentRouteV1)

GitLab: `#2309`

Intent Route v1 is the **policy contract** that maps:

1. **Intent** (task type + confidence + What/How/Do dimension)
2. **Budgets** (token + cost budget levels)
3. **Context pressure** (ledger utilization + pressure action)

→ into deterministic, traceable recommendations:

- **Model tier** (`fast|standard|premium`) with profile cap + budget-based degradation
- **Read mode** recommendation with pressure-based degradation
- **Reason** string (traceable, no secret inputs)

## Stability & versioning

- Payload includes `schema_version` (SSOT: `rust/src/core/contracts.rs`).
- Router decisions must be deterministic: same inputs + policy → same output.

## Security / privacy

- Raw query is not stored; the contract includes:
  - `query_md5`
  - a **redacted, bounded excerpt** (`query_redacted`)
- No secrets should appear in routing artifacts, logs, or exported proofs.

## Profile overrides

Profiles can cap routing via `routing.max_model_tier` and control degradation behavior:

- File: `rust/src/core/profiles.rs`
- Config section: `[routing]`

## Relevant code

- Router contract + policy: `rust/src/core/intent_router.rs`
- Legacy heuristic route (used as base signal): `rust/src/core/intent_engine.rs` (`route_intent`)

