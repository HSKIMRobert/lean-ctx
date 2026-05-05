# Knowledge Policy Contract v1

GitLab: `#2317`

## Ziel

Ein **versionierter Governance-Vertrag** für Project Knowledge (Facts/Patterns/History + Relations), der:

- **bounded** ist (keine unkontrollierte Growth/Outputs),
- **deterministisch** ist (stable ordering + stable semantics),
- **replayable** ist (Timeline/History bleibt interpretierbar),
- **sicher** ist (kein secret leakage, policy-gated surfaces),
- **auditable** ist (Writes emit events).

## Version (SSOT)

- `leanctx.contract.knowledge_policy_v1.schema_version=1`
  - SSOT: `CONTRACTS.md`
  - Runtime: `rust/src/core/contracts.rs`

## Policy Surface

### Config (global/project)

Policy wird über `Config.memory` geladen und validiert:

- Runtime: `rust/src/core/config.rs` (`memory_policy_effective`)
- Structs: `rust/src/core/memory_policy.rs`

Minimaler TOML-Ausschnitt:

```toml
[memory.knowledge]
max_facts = 200
max_patterns = 50
max_history = 100
contradiction_threshold = 0.5

# Retrieval budgets (bounded outputs)
recall_facts_limit = 10
rooms_limit = 25
timeline_limit = 25
relations_limit = 40

[memory.lifecycle]
decay_rate = 0.01
low_confidence_threshold = 0.3
stale_days = 30
similarity_threshold = 0.85
```

### Env Overrides (optional)

Die Runtime akzeptiert env overrides für zentrale Felder (siehe `memory_policy.rs`).

## Semantik (MUST)

### Facts

- Facts sind logisch durch `(category, key)` adressiert.
- Current vs archived:
  - current: `valid_until == None`
  - archived: `valid_until != None`
- Updates **dürfen** den vorherigen Zustand nicht “vergessen”: Timeline muss eine nachvollziehbare Folge liefern (archived versions + current).

### Contradictions

- Contradiction detection:
  - Case-insensitive equality ⇒ **kein** Widerspruch.
  - Word-similarity über Schwelle ⇒ **kein** Widerspruch (verhindert false positives bei semantisch gleichen Werten).
- Severity semantics (stabil):
  - `High`: `existing.confidence >= 0.9` **und** `existing.confirmation_count >= 2`
  - `Medium`: `existing.confidence >= contradiction_threshold`
  - `Low`: sonst

### Retrieval Budgets (bounded, deterministisch)

- `recall_facts_limit`: max facts in Recall Outputs
- `rooms_limit`: max rooms in `ctx_knowledge action=rooms`
- `timeline_limit`: max entries in `ctx_knowledge action=timeline`
- `relations_limit`: max edges in `ctx_knowledge action=relations|relations_diagram`

Ordering ist deterministisch (stable sort tie-breaks), danach erfolgt Truncation.

### Lifecycle

- Confidence decay + consolidation + compaction laufen deterministisch und bounded:
  - Runtime: `rust/src/core/memory_lifecycle.rs`
  - Parameter: `decay_rate`, `low_confidence_threshold`, `stale_days`, `similarity_threshold`, `max_facts`

## Tool Surface

- `ctx_knowledge` (MCP): `rust/src/tools/ctx_knowledge.rs`
  - `action=policy` + `value=show|validate` (policy visibility + range validation)
  - Writes (`remember`, `pattern`, `feedback`, relations) emit **audit events**
- Relations: `rust/src/tools/ctx_knowledge_relations.rs` (bounded + deterministic outputs)

## Security & Privacy

- Keine Secrets in Knowledge/Artifacts/Logs; Redaction + path boundaries gelten auch für Memory I/O.
- Outputs sind bounded; Policies müssen nicht zu Token-Burn führen.

