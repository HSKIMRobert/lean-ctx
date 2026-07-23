# System Patterns

## Architektur-Überblick

```
┌─────────────────────────────────────────────────┐
│ IDE (Cursor/Claude Code/VS Code/Windsurf/Zed)   │
└──────────────────────┬──────────────────────────┘
                       │ MCP stdio / HTTP
┌──────────────────────▼──────────────────────────┐
│ lean-ctx Server (rust/src/server/)               │
│  ├── ToolRegistry (81+ tools, SSOT)             │
│  ├── Loop Detection + Correction Tracking       │
│  ├── Role Guard + Workflow Gating               │
│  └── bounded_lock (adaptive timeout RwLock)     │
├─────────────────────────────────────────────────┤
│ Core Layer (rust/src/core/)                      │
│  ├── Session Cache (LRU, token-budget 500k)     │
│  ├── PathJail + IO Boundary                     │
│  ├── io_health (WSL2/NFS detection)             │
│  ├── Graph Index (4-layer edges)                │
│  ├── BM25 Full-Text Index                       │
│  ├── Consolidation (providers → BM25+Graph+Knowledge+Cache) │
│  ├── ProviderRegistry (GitHub/GitLab/Jira/Postgres/MCP/REST) │
│  ├── Shell Allowlist (AST-based parsing)        │
│  ├── ModePredictor (learned read modes)         │
│  ├── Savings Ledger (Ed25519-signed batches)    │
│  └── Memory Guard (RSS tiered eviction)         │
├─────────────────────────────────────────────────┤
│ Transport Layer                                  │
│  ├── MCP stdio (hybrid JSON-line + Content-Length) │
│  ├── Proxy (axum, 127.0.0.1, token auth)       │
│  ├── Gateway Server (MCP observe, admin API)    │
│  ├── HTTP Server (team, connectors, savings)    │
│  └── Dashboard (custom HTTP, CSP, CSRF)         │
└─────────────────────────────────────────────────┘
```

## OCLA Architecture (Betriebsgeheimnis)

### Drei Betriebsplanes

```text
Token Data Plane            Gateway · MCP · SDK · Sidecar · Shell
Token Control Plane         Policies · Budgets · Routing · Identity · Experimente
Token Value/Evidence Plane  Ledger · Outcomes · Savings · Audit · Approval
```

Die drei Planes sind customer-owned. Der Data Plane hängt nie von Thinkery
Cloud, AI Value Gate oder einem Lizenzserver ab.

### 5-Schichten-Architektur (SOLL)

```
Agents & Apps (Cursor, Claude Code, Custom, Multi-Agent)
  |
  +- Schicht 5: AI VALUE GATE / ENTERPRISE SUBSCRIPTION (Commercial)
  |     Control Plane, Value Intelligence, Assurance, SLA, Support
  +- Schicht 4: INTERCEPTION POINTS (OSS)
  |     LeanCTX (Coding Agents) . Lean Embed (SDK/API) . Lean OS (Enterprise)
  +- Schicht 3: UNIFIED LEDGER
  |     Ed25519 + SHA-256, Messmethode, Qualität, Attribution, Approval
  +- Schicht 2: OCLA CONTRACT (OSS) -- 14 Traits, 4 Dimensionen
  |     OBSERVE(6) . CONTROL(3) . LEARN(1) . ACT(2) . SHARED(2)
  +- Schicht 1: THE ENGINE (OSS) -- lean-ctx-core
  |     81+ MCP Tools, Proxy, CLI, Shell Engine, Caches, Indexes
  |
  LLM Providers (OpenAI, Anthropic, Google, Azure, Local)
```

### 14 OCLA Traits

```
OBSERVE (6 Traits):
  ObservationHook   — pre/post request lifecycle
  UsageSink         — token/cost recording
  MetricsExporter   — JSON/Prometheus/OTEL export
  SavingsLedger     — Ed25519-signed savings (revenue basis)
  IntentClassifier  — NEU: persona/intent-tag pro request (DIM 3)
  OutcomeTracker    — NEU: output used/rejected? (AI Value Multiple)

CONTROL (3 Traits):
  CompressionProvider — input compression + CCR (DIM 1)
  ResponseOptimizer   — NEU: output optimization, verbosity (DIM 2)
  ModelRouter         — NEU: cost-optimal model routing (DIM 3)

LEARN (1 Trait, LOCAL-FIRST):
  EfficiencyAnalyzer  — lokale analyse, read-only

ACT (2 Traits):
  ConfigTuner      — config changes with approval + rollback
  ExperimentRunner — A/B test + auto-rollback

SHARED (2 Traits):
  ConnectorScheduler — external data (GitHub/GitLab/Jira)
  AgentGateway       — NEU: multi-agent token control (DIM 4)
```

### 4 Kontroll-Dimensionen (Chiptuner-Analogie)

| DIM | Analogie | Was | Trait | Status |
|-----|----------|-----|-------|--------|
| 1 | Einspritzung (Input) | Handles/Deltas + quality-bound Compression | CompressionProvider | GEBAUT/PARTIAL |
| 2 | Abgas (Output) | minimal vollständiger typed Output | ResponseOptimizer | Phase 9 |
| 3 | Leistungsstufe (Routing) | kleinstes qualifiziertes Modell/Effort | ModelRouter + IntentClassifier | Phase 8 |
| 4 | Mehrzylinder (Agent) | Capsules, Ownership, Work Graph, Chain Budget | AgentGateway | Phase 11 |

### OSS / Commercial Grenze

GitHub `yvgude/lean-ctx` bleibt vollständig Apache-2.0. Commercial Code lebt
von Beginn an in separaten privaten GitLab-Repositories:

| Repository | Lizenz | Inhalt |
|-------|--------|--------|
| `yvgude/lean-ctx` | Apache-2.0 | Engine, MCP, Proxy, Gateway, OCLA, SDKs, Ledger |
| `root/lean-ctx-enterprise` | Proprietary | AI Value Gate + Enterprise Control Plane |
| `root/lean-ctx-cloud` | Proprietary | SaaS, Billing, Sync, License Issuance |
| `root/lean-ctx-deploy` | Proprietary | Deployment Factory |
| `root/<customer>-deploy` | Customer-private | Values/Digests/Secret-Refs; kein Code |

Lokale Basisimplementierungen aller 14 Capabilities bleiben OSS. Commercial
sind organisationsweite Intelligence, Fleet/Policy Management, SSO/SCIM, LTS,
Assurance, Compliance, SLA und Services. Kundendaten bleiben kundeneigen.

### OCLA Dual Contract

- Rust Traits für in-process/zero-cost Integration
- versionierter Wire Contract für REST/gRPC und Partner SDKs
- Capability Discovery erlaubt Teilimplementierungen
- Contract-Version, Error Model, Deadlines, Idempotency, Backpressure
- externer Example Consumer + gemeinsame Contract Suite

### Canonical Token Envelope

- providerneutrale Request/Message/Content/Tool/Usage/Error-Semantik
- Streaming Chunks, Finish Reasons, Cancellation und Backpressure explizit
- Adapter melden unsupported/lossy; stilles Droppen verboten
- Golden Traces beweisen Cross-Protocol-Fidelity
- Policy, Routing, Compression und Evidence arbeiten gegen gemeinsame Typen

### Savings Evidence

- Ed25519/SHA-256 beweisen Integrität und Herkunft, nicht Kausalität
- direct/holdout/baseline/estimate/customer-confirmed getrennt
- Quality Gate + exklusive Attribution + Customer Approval
- keine doppelte Attribution zwischen Compression/Caching/Routing/Output

### Migrations-Pattern: Strangler Fig

Pro Trait ein MR. Bestehende direkte Aufrufe werden durch Trait-Calls ersetzt.
Gleicher Code laeuft dahinter (Wrapper, kein Rewrite). Benchmark-Gate: > 2% Regression blockt MR.
Dispatch: Generics lokal (zero-cost), `dyn Trait` an Gateway-Grenze.

Enterprise-Einführung: `OBSERVE → MEASURE → CONTROL → OPTIMIZE → AUTOMATE`.
P7 Wire Contract/SDKs ist REQUIRED; P6 physische Trennung bleibt deferred.

Effizienz-North-Star: `Effective Tokens per Accepted Outcome` (ETPAO) bilanziert
Input, reported Reasoning, Output, Tool-Schema/Instructions, Cache, Retries,
Re-Reads, Regenerierungen und A2A. Clientpfade tragen eine runtime-belegte
Coverage Class. Wiederholter Kontext fließt über Handles/Deltas; Agenten teilen
`ContextCapsuleV1` statt vollständiger Denk-/Dateihistorie.

Programmsteuerung: W0–W10/G0–G10 bilden Gesamtumbau bis GA; P0–P11 bleiben
technische OCLA Work-Packages. Completion benötigt Requirement→Evidence und ein
zweites kundenähnliches Deployment ohne Code-Fork.

### Repository und Delivery Boundary

```text
GitHub lean-ctx (Apache-2.0 Source)
  ├─ versioned OCLA/Envelope/SDK contracts ─→ lean-ctx-enterprise (private)
  ├─ versioned cloud contracts ─────────────→ lean-ctx-cloud (private)
  └─ signed OSS image ──────────────────────→ lean-ctx-deploy (private)
                                                └→ customer-deploy overlay
```

- GitLab `root/lean-ctx` ist read-only OSS Mirror, nicht Commercial Monorepo
- private Komponenten importieren keine internen OSS-Runtime-Typen
- Website wird in eigenes privates Repository extrahiert
- CI erzeugt SBOM, Provenance, Signatur und immutable OCI Digests
- Deploy Factory und Customer Overlay pinnen Digests und Contract-Versionen
- Server erhält Images/Config, keine manuell gepflegten Source Trees
- vollständiger SSOT: `docs/business/gateway-integration/repository-delivery-boundary.md`

### Holistic Context Control

```text
MCP/Proxy/Shell/Provider/Agent
  → Candidate Fabric (Session, Knowledge, Cache, Search/Graph, Memory, Evidence)
  → Context Control Kernel (policy → Φ → budget → view/order/recovery)
  → ContextPlanV1 → execution → ContextReceiptV1
  → outcome/quality → learn/consolidate/invalidate
```

- Stores bleiben getrennt; `ContextObjectV1` vereinheitlicht nur Planungsmetadaten
- Context Field/Compiler wird gemeinsamer Runtime-Service, nicht nur Tool-Surface
- ContentRef/Handle/Delta ist stale-safe, tenantgebunden und wire-fähig
- Outcome Learning respektiert Receipt-Lineage und exklusive Attribution
- SSOT: `docs/business/gateway-integration/holistic-context-intelligence.md`

Details: Master-Plan `plans/ist-zu-soll_ocla_migration_9ff31402.plan.md` + Canvas `canvases/ocla-soll-architektur.canvas.tsx`

## Code-Struktur

- **`rust/src/core/`**: Datenstrukturen, Algorithmen, Caches, Config
- **`rust/src/tools/registered/`**: MCP Tool Implementierungen (McpTool trait)
- **`rust/src/server/`**: MCP Server, Dispatch, Registry, bounded_lock
- **`rust/src/cli/`**: CLI Commands
- **`rust/src/dashboard/`**: Web Dashboard mit Routes
- **`rust/src/proxy/`**: HTTP Proxy (axum-based)
- **`rust/src/gateway_server/`**: Gateway Server (MCP observe, admin)
- **`rust/src/http_server/`**: Team Server, Connectors, Savings
- **`rust/src/cloud_server/`**: Cloud Server (auth, billing, sync)
- **`rust/src/shell/`**: Shell compression engine

## Schlüssel-Patterns

### Tool Registration (SSOT)
- `ToolRegistry` in `server/registry.rs` ist Single Source of Truth
- Test `test_registry_tool_count_ssot` verhindert Drift

### Self-Healing I/O
- `io_health.rs`: Erkennt IoEnvironment (Fast/SlowFs/Degraded)
- `bounded_lock.rs`: RwLock mit adaptive Timeouts
- Pattern: Lock-Timeout → Fallback (statt Hang)

### Multi-Layer Graph
- Layer 1: Import Edges (weight 1.0)
- Layer 2: Implicit Language Edges (weight 0.8)
- Layer 3: Co-Change Edges (weight 0.5)
- Layer 4: Sibling Edges (weight 0.2)

### Security Layers
1. PathJail (project root + allow_paths)
2. Shell Allowlist (AST-parsed, all segments must match)
3. Env Filtering (LD_PRELOAD etc. blocked)
4. Token Auth (CSPRNG, constant-time, 0600)
5. Host Guard (DNS rebinding protection)
6. IO Boundary (secret detection, path validation)

### Process Management
- macOS: LaunchAgent mit KeepAlive=true
- `lean-ctx stop`: unload + SIGTERM + cleanup
- `lean-ctx dev-install`: stop → build → atomic install → restart

### Provider Consolidation Pipeline
- `ContextProvider::execute()` → `ProviderResult` → `consolidation::consolidate()` → `ConsolidationArtifacts`
- `apply_artifacts_to_stores()` [Background-Thread]: BM25 + Graph + Knowledge + Cache

## Anti-Patterns (vermeiden)

- `kill` ohne `launchctl unload` (respawnt sofort)
- `blocking_read()`/`blocking_write()` ohne Timeout
- Hardcoded Port 4444 (→ `default_port()` nutzen)
- `std::fs::canonicalize` direkt (→ `safe_canonicalize_bounded`)
- Token in Logs/URLs ohne Maskierung
