# Lock Ordering — lean-ctx Rust Codebase

This document catalogues every global/static lock and notable `Arc<Mutex/RwLock>` in the
codebase, defines the intended acquisition order, and records rules for async code.

---

## 1. Global / Static Locks

All `std::sync::Mutex` unless noted otherwise.

| # | Lock | File | Type | Purpose |
|---|------|------|------|---------|
| L1 | `REGISTRY` | `core/index_orchestrator.rs:57` | `OnceLock<Mutex<HashMap<String, Arc<Mutex<ProjectBuild>>>>>` | Outer map of per-project build state |
| L2 | per-project `ProjectBuild` | `core/index_orchestrator.rs:57` (inner) | `Arc<Mutex<ProjectBuild>>` | Individual project build progress |
| L3 | `HEATMAP_BUFFER` | `core/heatmap.rs:10` | `Mutex<Option<HeatMap>>` | Buffered access-frequency heatmap |
| L4 | `Config::CACHE` | `core/config/mod.rs:885` | `Mutex<Option<(Config, SystemTime, Option<SystemTime>)>>` | Config file cache with mtime check |
| L5 | `FEEDBACK_BUFFER` | `core/feedback.rs:9` | `Mutex<Option<(FeedbackStore, Instant)>>` | Buffered user feedback |
| L6 | `PREDICTOR_BUFFER` | `core/mode_predictor.rs:8` | `Mutex<Option<(Arc<ModePredictor>, Instant)>>` | Cached mode predictor model |
| L7 | `STATS_BUFFER` | `core/stats/mod.rs:13` | `Mutex<Option<(StatsStore, StatsStore, Instant)>>` | Token-savings statistics |
| L8 | `COST_BUFFER` | `core/a2a/cost_attribution.rs:69` | `Mutex<Option<CostStore>>` | A2A cost tracking |
| L9 | `GLOBAL_LIMITER` | `core/a2a/rate_limiter.rs:121` | `Mutex<Option<RateLimiter>>` | Global A2A rate limiter |
| L10 | `DETECTOR` | `core/anomaly.rs:222` | `OnceLock<Mutex<AnomalyDetector>>` | Anomaly detection state |
| L11 | `SLO_CONFIG` | `core/slo.rs:101` | `OnceLock<Mutex<Vec<SloDefinition>>>` | SLO definitions |
| L12 | `VIOLATION_LOG` | `core/slo.rs:102` | `OnceLock<Mutex<ViolationHistory>>` | SLO violation history |
| L13 | `EMIT_STATE` | `core/slo.rs:103` | `OnceLock<Mutex<HashMap<String, EmitState>>>` | SLO emission dedup state |
| L14 | `ACTIVE_ROLE_NAME` | `core/roles.rs:12` | `OnceLock<Mutex<String>>` | Currently active role name |
| L15 | `PROVIDER_CACHE` | `core/providers/cache.rs:5` | `LazyLock<Mutex<ProviderCache>>` | Cached provider metadata |
| L16 | `LAST_BANDIT_ARM` | `core/adaptive_thresholds.rs:337` | `Mutex<Option<(String, String, String)>>` | Last bandit arm selection for adaptive thresholds |

### Test / Environment Locks (serialise env-var mutations)

| # | Lock | File | Purpose |
|---|------|------|---------|
| E1 | `ENV_LOCK` | `dashboard/mod.rs:537` | Serialize env-var access in dashboard tests |
| E2 | `ENV_LOCK` | `core/dense_backend.rs:412` | Serialize env-var access in dense-backend tests |
| E3 | `ENV_LOCK` | `core/workspace_config.rs:101` | Serialize env-var access in workspace-config tests |
| E4 | `LOCK` | `core/data_dir.rs:50` | Serialize data-dir creation |
| E5 | `LOCK` | `core/tokens.rs:190` | Serialize tokenizer tests |
| E6 | `LOCK` | `core/tokenizer_translation_driver.rs:248` | Serialize tokenizer-translation tests |

---

## 2. Arc-wrapped Session Locks (per-MCP-session, `tokio::sync::RwLock`)

Defined in `tools/mod.rs` on `ToolContext`:

| Field | Type | Purpose |
|-------|------|---------|
| `cache` | `Arc<RwLock<SessionCache>>` | File content cache |
| `session` | `Arc<RwLock<SessionState>>` | Session metadata |
| `tool_calls` | `Arc<RwLock<Vec<ToolCallRecord>>>` | Call log |
| `last_call` | `Arc<RwLock<Instant>>` | Idle-timeout tracking |
| `agent_id` | `Arc<RwLock<Option<String>>>` | Current agent identifier |
| `client_name` | `Arc<RwLock<String>>` | Connected client name |
| `loop_detector` | `Arc<RwLock<LoopDetector>>` | Loop-detection state |
| `workflow` | `Arc<RwLock<Option<WorkflowRun>>>` | Active workflow run |
| `ledger` | `Arc<RwLock<ContextLedger>>` | Context ledger |
| `pipeline_stats` | `Arc<RwLock<PipelineStats>>` | Pipeline statistics |
| `context_ir` | `Option<Arc<RwLock<ContextIrV1>>>` | Context IR state |

These are all **`tokio::sync::RwLock`** and are scoped to a single session — no cross-session
nesting is expected. Within a single tool handler, acquire at most one at a time.

### Other Arc-wrapped Locks

| Lock | File | Type | Purpose |
|------|------|------|---------|
| `SharedProtocol` | `mcp_stdio.rs:30` | `Arc<Mutex<Option<WireProtocol>>>` | MCP stdio wire protocol (std::sync) |
| `SharedSessions.session` | `core/context_os/shared_sessions.rs:31` | `Arc<tokio::sync::RwLock<SessionState>>` | Shared session state across channels |

---

## 3. Lock Acquisition Order

### Rule: always acquire outer → inner, lower number → higher number.

```
L1 (REGISTRY outer map)
 └─► L2 (per-project ProjectBuild)     — NEVER hold L1 while locking L2
```

The `entry_for()` function in `index_orchestrator.rs` enforces this: it locks L1, clones the
`Arc<Mutex<ProjectBuild>>`, **drops** L1, then the caller locks L2 independently. This avoids
deadlock by ensuring L1 and L2 are never held simultaneously.

### Independent Static Locks (L3–L16)

All other static locks (L3–L16) are **independent singletons** — they protect isolated subsystem
state and are never nested inside each other. Each should be acquired in isolation:

- **Do not hold two static locks at the same time.** If a future change requires locking two
  subsystems, add the ordering rule here first.
- **Hold locks for the minimum duration.** Clone/copy data out, drop the guard, then do work.

### Session Locks (`tokio::sync::RwLock`)

Session-scoped `RwLock`s on `ToolContext` are logically independent:

- Acquire at most **one session lock per tool handler** at a time.
- If you must acquire two, acquire in field-declaration order (cache → session → tool_calls → …).
- **Never hold a session RwLock while locking a global static Mutex** — this risks priority
  inversion between the tokio runtime and OS threads.

### Test/Environment Locks (E1–E6)

These exist solely to serialise tests that mutate environment variables. They must not be held
across any other lock acquisition.

---

## 4. Async Code: `tokio::sync::Mutex` vs `std::sync::Mutex`

| Use | When |
|-----|------|
| `std::sync::Mutex` | Lock held briefly (no `.await` while held), data is `Send` only, or lock is static/global |
| `tokio::sync::Mutex` | Lock must be held **across** `.await` points, or guards must be `Send` for spawned futures |
| `tokio::sync::RwLock` | Readers dominate, writers are rare; lock may be held across `.await` |

### Current usage

- **Global statics** → all `std::sync::Mutex` (correct: locks are held for microseconds, no await)
- **HTTP rate limiter** (`http_server/mod.rs`) → `tokio::sync::Mutex` (correct: held in async handler)
- **Team audit file** (`http_server/team.rs`) → `tokio::sync::Mutex` (correct: held across `tokio::fs::File` writes)
- **Session state** (`tools/mod.rs`) → `tokio::sync::RwLock` (correct: accessed from async tool handlers)
- **Shared sessions** (`core/context_os/shared_sessions.rs`) → `tokio::sync::RwLock` (correct: shared across async channels)

### Rules

1. **Never `.await` while holding a `std::sync::Mutex` guard.** The tokio runtime thread will
   block, starving other tasks.
2. **Prefer `std::sync::Mutex` for global caches** where the critical section is a quick
   read/write with no I/O.
3. **Use `tokio::sync::Mutex` only when the critical section contains `.await`.**
4. A `std::sync::MutexGuard` is `!Send` — you cannot hold it across an `.await` even if you
   wanted to. The compiler enforces this.

---

## 5. Adding New Locks — Checklist

1. Determine scope: global static vs per-session vs per-request.
2. Choose `std::sync` vs `tokio::sync` per Section 4.
3. Assign a lock number (append to Section 1) and document the acquisition order here.
4. If nesting is required, document the outer → inner relationship in Section 3.
5. Run `cargo check --all-features` to verify `Send`/`Sync` bounds.
