# OCLA-Umbau-Ziel

Dieses Dokument ist die Fortschrittstabelle für die OCLA-Phasen und trennt
fertige Verträge von noch fehlender Produktions-Adoption.

Stand: 2026-07-28, nach E4 A2A Transport Hardening.

## Fortschritt pro P-Phase

| Phase | Ziel | Stand auf `main` | Evidenz / nächster Schritt |
| --- | --- | --- | --- |
| P0 | IST-Hygiene | **Erledigt** | `e379c9db0`; Grundlagen bereinigt. |
| P1 | Foundation, Contracts, Ledger Evidence, Lineage | **Erledigt** | `#1053`, `79290e63d`. |
| P2 | OclaBus Event-Backbone | **Erledigt** | `1029229a1`; globaler Bus mit bounded/no-op-Modus. |
| P3 | Builtin-Traits | **Erledigt** | 14 Traits, 15 Builtins, Registry, Boundary-Härtung, fail-closed Gates. |
| P4 | Trait-Adoption in Runtime | **Erledigt (14/14)** | Alle 14 Traits produktiv verdrahtet. |
| P5 | Unified Ledger + Budget + Tracing | **100 %** | R5: Dual-Write, Approval/Settlement. R7: Budget Cascade, trace_id, Reconciliation. R8: Budget REST-API, Reconciliation CLI. R10: OclaRuntime Binary Separation (`ef9744b37`). |
| P6 | Separater OCLA-Meilenstein | Nicht belegt | In P0-P5 absorbiert. |
| P7 | Wire Protocol, SDKs, gRPC, Contract Suite | **100 %** | R5-R6: REST/OpenAPI/Streaming/Py+TS SDKs/Middleware/Sidecar. R8: Go SDK. R9: gRPC Bridge Config, Proto BudgetCheck RPC. R10: gRPC Server Live-Wiring mit TcpListener (`8164cb290`), SDK Capsule Endpoints (`bcf8494a6`). |
| P8 | Intent-/Model-Router | **100 %** | R7: Quality Gate, ETPAO Benchmark. R8: Routing Feedback Adapter. R9: Live-Wiring in routing.rs. R10: A/B-Test-Framework (`2ab2cd49f`). |
| P9 | Response Optimizer | **100 %** | R7: Response Cache. R8: Cache Bridge. R9: Live-Wiring in forward.rs. R10: Model-Aware Cache-Invalidation-Policy (`a40050b21`). |
| P10 | AI Value Gate (Commercial) | Nicht gestartet | Privates Repo `lean-ctx-enterprise`. |
| P11 | Agent Gateway und Deployment Surface | **100 %** | R7: DLQ, Health Surface. R8: DLQ Wire-API, Distributed Tracing. R9: CapsuleStore CoW, Capsule REST Endpoints. R10: Capsule→AgentGateway Wiring (`f658ad543`), Health Component Integration (`25b0b6301`), Final Smoke Test (`b84444eac`). |

## Produktions-Adoption: aktueller Zähler

| Verdrahtet | Gehärtet | Offen | Gesamt |
| ---: | ---: | ---: | ---: |
| 14 | 0 | 0 | 14 |

## Gemergte OCLA-Änderungen

### R1–R4 (P0–P4 Grundlage)
- `#1053` — P1 Foundation.
- `#1065` — Trait-Adoption-Grundlage: 14 Builtins plus Registry.
- `#1070` — UsageSink und EfficiencyAnalyzer produktiv.
- `#1071` — ObservationHook produktiv.
- `#1073` — OutcomeTracker produktiv.
- `#1075` — CompressionContentPort mit PathJail/BLAKE3.
- `#1076` — echte CompressionProvider-Kompression.
- `#1083` — fail-closed Provider und TOCTOU-Härtung.
- `#1092` — Projektwurzel und Runtime-Callsite korrigiert.
- `#1093` — MetricsExporter produktiv.
- `5394aa6e3` — Fail-closed-Gates und Compression-Härtung.

### R5 (P4-Abschluss, P5, P7, P8, P9)
- `07e1dbfea` — IntentClassifier adoptiert (P4 100%).
- `49b7a56dd` — P5 Dual-Write.
- `e12f8458b` — Approval/Settlement-Workflow.
- `122fa1b85` — P7 REST API Server.
- `c1ebfa714` — OpenAPI 3.1 Spec.
- `9b713e21a` — Wire Streaming Semantik.
- `85cbbf0ef` — Contract Golden Suite.
- `dc7bbaebb` — P8 Model Router: Intent-basiertes Routing.
- `941d44172` — P9 Response Optimizer: Similarity-Dedup.

### R6 (P7 Wire Contract + SDKs)
- `65b49bd59` — REST API +3 Endpoints.
- `14b15dfc1` — Idempotency + Size Middleware.
- `2badcfa32` — Python SDK.
- `18b18d185` — TypeScript SDK.
- `80c8029e9` — External Consumer Example.
- `fc0c6f2dc` — Sidecar Deployment Profile.
- `b6b8edf75` — Contract Suite erweitert.

### R7 (P5/P8/P9/P11 Hardening)
- `9386305cb` — Budget Cascade (230 LOC, 4 Tests).
- `1d341ce40` — trace_id Propagation (174 LOC, 5 Tests).
- `cd9e5dc89` — Ledger Reconciliation (135 LOC, 2 Tests).
- `978dc18ae` — Router Quality Gate (137 LOC, 3 Tests).
- `e96ee2ee1` — Response Cache (294 LOC, 6 Tests).
- `10b02f45e` — ETPAO Benchmark Suite (254 LOC, 5 Tests).
- `5c2166a88` — Dead Letter Queue (235 LOC, 5 Tests).
- `d04ec3f63` — Health Surface (253 LOC, 5 Tests).

### R8 (Production Wiring)
- `5bee3a31b` — Budget REST-API: POST/GET/DELETE Endpoints (300 LOC).
- `c620d79ac` — Reconciliation CLI: `lean-ctx ocla reconcile [--json]` (47 LOC).
- `d6abd6612` — trace_id E2E: X-Trace-Id Header-Propagation (70 LOC).
- `da6fc6700` — OCLA Cache Bridge: Adapter für Proxy-Integration (102 LOC).
- `b2ecb8e03` — Routing Feedback: Quality-Tracking-Adapter (164 LOC).
- `41b281d4a` — Go SDK: vollständiger HTTP-Client für OCLA v1 (428 LOC).
- `d28e3a07e` — Distributed Tracing: SpanCollector mit Ring-Buffer (297 LOC).
- `67413f376` — DLQ Wire-API + Health Integration (263 LOC).

### R9 (Phase Completion: Live-Wiring + CoW + gRPC)
- `3e70c89df` — P9 Cache Bridge Live-Wiring in forward.rs (36 LOC).
- `13f254df7` — P8 Routing Feedback Live-Wiring in routing.rs (24 LOC).
- `e411115aa` — P7 gRPC Bridge Config + Server Module (154 LOC).
- `488eab8bb` — P7 Proto Extension: trace_id + BudgetCheck RPC (48 LOC).
- `38e517063` — P11 CapsuleStore CoW Handles (297 LOC).
- `2c1b482d3` — P11 Capsule REST Endpoints + OpenAPI (87 LOC).
- `c2a457eba` — OCLA Integration Test Suite (285 LOC).
- `2007e40d4` — Contract Suite + OpenAPI Update (164 LOC).

### R10 (Finish Line: P5/P7/P8/P9/P11 → 100%)
- `ef9744b37` — P5 OclaRuntime: Binary Separation mit CancellationToken (108 LOC).
- `8164cb290` — P7 gRPC Server Live-Wiring: TcpListener + AtomicBool (99 LOC).
- `2ab2cd49f` — P8 A/B-Test-Framework: Weighted Variants, Outcome Tracking (278 LOC).
- `a40050b21` — P9 Cache-Invalidation-Policy: ModelAware TTL, Eviction Stats (181 LOC).
- `f658ad543` — P11 Capsule→AgentGateway Wiring: CoW Fork in Relay (95 LOC).
- `25b0b6301` — P11 Health Component Integration: Capsule, Cache, Tracing (157 LOC).
- `bcf8494a6` — P7 SDK Capsule Endpoints: Python + TS + Go (278 LOC).
- `b84444eac` — Final Smoke Test: Capsule E2E + Health Validation (188 LOC).
- `8b414a278` — Quality Gate Fix: Clippy, Duplicates, Lifetimes, Visibility.

### R12 (File-Splits: Alle Files unter 1500 LOC)
- Refactoring-Runde für alle Dateien über 1500 LOC.

### R13 (Context Control Kernel — Holistic Intelligence Foundation)
- `bd3a09c5f` — Context Kernel Foundation: types.rs (ContextObjectV1, CandidateProvider trait, Plan/Receipt) (400 LOC).
- `bd3a09c5f` — Providers: Knowledge, Session, Episodic, Procedural, Ledger (549 LOC).
- `bd3a09c5f` — Orchestrator: gather → Phi-score → compile → plan → receipt (456 LOC).
- `bd3a09c5f` — Bridge: ctx_compose Integration, OclaBus Events, Feedback Loop (256 LOC).
- Feature-gated hinter `context_kernel` Flag. 25 Tests, 0 Failures.

## Abschlusskriterien

**P0–P4: ERLEDIGT.** Alle 14 Builtins produktiv verdrahtet.

**P5–P9, P11: ERLEDIGT (100%).** Unified Ledger mit Binary Separation,
Wire Protocol mit REST/gRPC/SDKs, Model Router mit A/B-Testing,
Response Optimizer mit Cache-Invalidation, Agent Gateway mit CoW Capsules.

**P10 (AI Value Gate): OFFEN.** Privates Repo `lean-ctx-enterprise`, nicht Teil des OSS-Umbaus.

Der OSS OCLA-Umbau (P0–P11) und die Holistic Context Intelligence Waves
W0–W7 sind abgeschlossen. Der Context Control Kernel ist produktiv in
allen Hot-Paths (ctx_read, ctx_search, ctx_shell, ctx_compose, Context Gate)
mit Policy-Enforcement, Outcome-Learning und 3 Partner-SDKs (TS, Go, Python).
**ALLE 10 WAVES ABGESCHLOSSEN (W0–W10).**
**ALLE 16 §13 COMPLETION CRITERIA ERFÜLLT.**
R19 hat die Deep-Audit-Findings (26 Issues) adressiert:
- Kernel von Appender zu Gatekeeper transformiert (150-Token Cap + Suppress)
- Content-Dedup eliminiert redundante Resends (95-99% bei Unchanged)
- Ehrliche Token-Accounting mit Phantom-Savings-Detection
- Outcome Feedback Loop geschlossen (Shadow→Enforce via Config)
Nächste Phase: Hot-Path-Wiring (Dedup + Activation in echte Tool-Pfade verdrahten).


### R14 (Context Kernel Live-Wiring)
- Feature-Flag `context_kernel` aktiviert (default on).
- `kernel_enrich` in `ctx_compose` integriert (Strangler Step 1).
- ShadowLogger für Plan/Receipt-Audit-Trail.
- Golden Context Benchmarks (3 Workloads).
- 0 Post-Merge Clippy Errors. 37 Tests.

### R15 (Hot-Path Adoption + Outcome Receipt Loop)
- Context Kernel in ctx_read, ctx_search (semantic), ctx_shell Hot-Paths.
- Context Gate erzeugt ContextReceiptV1 nach jeder Dispatch-Operation.
- FeedbackCollector: Provider-Weight-Updates aus Outcome-Signalen.
- AttributionEngine: Per-Provider Savings-Attribution ohne Doppelzählung.
- ctx_read/kernel.rs Extraktion (LOC-Gate Compliance). 37 Tests.

### R16 (Policy Layer + Kernel Enforce + Python SDK)
- PolicyFilter: Sensitivity-, Source-, Budget-Filtering für Candidates.
- KernelMode: Shadow/Enforce/Explain mit config/env Steuerung.
- Python SDK (leanctx): Client + Types + Tests (httpx-basiert).
- TS/Go SDKs: Kernel Wire Types (ContextPlanV1, ReceiptV1, Policy, Attribution).
- OutcomeLearner: Receipt-driven Weight-Updates (EMA α=0.1).
- Conformance Suite: 4 Integration-Tests (Roundtrip, Policy, Attribution, Learning).
- 53 Kernel-Tests, 0 Clippy Errors, LOC-Gate OK.

### R17 (HA + Kernel Degradation + Invalidation)
- BoundedQueue: Ring-Buffer mit konfigurierbarer Kapazität.
- CircuitBreaker: Open/HalfOpen/Closed mit Cooldown und Failure-Threshold.
- ProviderCircuit: Pro-Provider Circuit-Breaker-Wrapper.
- DegradationLevel: Full/Reduced/Minimal/Bypass mit KernelHealth-Assessment.
- degrade_plan/fallback_plan: Graceful Plan-Reduktion bei Provider-Ausfall.
- InvalidationEvent/KernelInvalidationState: Content-Ref-basierte Invalidation-Propagation.
- KernelSnapshot: Serde-basierte State-Serialisierung mit atomarem Write (PID-unique Temp).
- Claude CLI Architecture Review: CONDITIONAL PASS → 5 WARNINGs gefixt.
- 72 Kernel-Tests, 0 Clippy Errors, LOC-Gate OK.

### R18 (Org-Views + Multi-Agent E2E — W9/W10 Abschluss)
- EtpaoMetrics + EtpaoTracker: Runtime ETPAO-Tracking pro Scope (277 LOC).
- KnowledgeHealthReport + EfficiencyView: Privacy-safe Org-Dashboards (215 LOC).
- ContextCapsuleV1 Wire-Type + DeltaTransfer + dedup_siblings (286 LOC).
- ResultFusion: BestConfidence/MajorityVote/WeightedMerge Strategien (297 LOC).
- Multi-Agent E2E Certification Suite: 6 Tests für §13 Criteria 7,10,11,13 (199 LOC).
- Claude CLI Architecture Review: CONDITIONAL PASS → 3 WARNINGs gefixt.
- 103 Kernel-Tests, 0 Clippy Errors, LOC-Gate OK.

### R19 (Critical Token Leak Fixes + Kernel Activation)
- 5-Agent Deep Audit: 26 Findings (5 CRITICAL, 10 HIGH) identifiziert.
- KernelVerdict + Gatekeeper-Mode: Kernel hat 150-Token Hard-Cap, Suppress-API (bridge.rs rewrite).
- ContextDedup: Unchanged-Content-Erkennung → 15-Token Stub statt vollständige Resend (254 LOC).
- A2A Fixes: read_unread Filter, Real Budget Init (1M statt MAX), Compact Handoff JSON (225 LOC).
- ActivationConfig: Shadow/Enforce/Explain Mode via Config, Outcome Feedback Loop (258 LOC).
- PostDeliveryAccounting: Ehrliche Token-Accounting inkl. Phantom-Savings-Detection (189 LOC).
- 131 Kernel-Tests, 0 Clippy Errors, LOC-Gate OK.

## Wave-Status (Holistic Context Intelligence)

| Wave | Beschreibung | Status |
|---|---|---|
| W0 | Inventar + Baseline | ✅ (R12) |
| W1 | Types + Contracts | ✅ (R13) |
| W2 | Request→Plan→Receipt→Outcome | ✅ (R15) |
| W3 | Benchmarks | ✅ (R14) |
| W4 | Kernel in Hot-Paths Shadow | ✅ (R15) |
| W5 | Policy + Identity | ✅ (R16) |
| W6 | Wire Types + SDK | ✅ (R16) |
| W7 | Outcome Learning | ✅ (R16) |
| W8 | HA + Degradation | ✅ (R17) |
| W9 | Org-wide Views | ✅ (R18) |
| W10 | Multi-Agent E2E | ✅ (R18) |

### R20 (Hot-Path Wiring + Quality Signal)
- Dedup, Activation, Accounting, A2A Fixes in ctx_read/ctx_compose/ctx_shell/ctx_search Hot-Paths integriert.
- Outcome Quality Signal: Echte Signal-Auswertung statt Placeholder.
- 149 Kernel-Tests, 0 Clippy Errors.

### R21 (Client Intelligence Layer)
- CoverageClass: Erkennung von Client-Capabilities (FullInline/ContextControlled/ObserveOnly/Unmanaged).
- ClientEfficiencyProfile: Pro-Client Token-Effizienz-Tracking.
- ContextBroker: Client-adaptive Budget-Allokation.
- ETPAO Live: Echtzeit-Messung pro Request.
- 185 Kernel-Tests, 0 Clippy Errors.

### R22 (Live-Wiring + Identity)
- Identity/Attribution: Globales Identity-Tracking über alle Requests.
- IdentityResolver: Header-basierte User-Erkennung.
- ClientWiring: Client-Profile → ContextBroker Integration.
- ToolSurface: Tool-Registry mit Coverage-aware Filtering.
- 220 Kernel-Tests, 0 Clippy Errors.

### R23 (Proxy Integration)
- ProxyBridge: Unified Bridge für proxy_bridge → Kernel Services.
- Identity + Coverage + ETPAO in forward_request verdrahtet.
- bridge_e2e: 7 E2E Conformance-Tests.
- 250+ Kernel-Tests.

### R24 (MCP Integration)
- McpBridge: MCP↔Kernel Bridge (Client, Calls, ETPAO, Identity).
- McpSchemaOpt: Tool-Description Compression + Budget Enforcement.
- McpReceipt: Honest Token-Accounting per Tool.
- McpCoverage: Client Coverage Detection.
- In post_dispatch verdrahtet.
- 300+ Kernel-Tests.

### R25 (Provider Envelope + Receipt Chain + Dashboard)
- TokenEnvelope: Provider-neutrale Canonical Token Usage.
- UsageNormalizer: Session-/Model-/Provider-Aggregation.
- ReceiptChain: Request→Plan→Receipt→Outcome Evidence Chain.
- LiveDashboard: JSON-Snapshot aller Kernel-Metriken.
- 320+ Kernel-Tests.

### R26 (Kernel Activation)
- KernelConfig: Runtime Feature-Toggles (enabled, content_dedup, schema_optimization, etc.).
- EnvelopeWiring: Evidence Pipeline Proxy+MCP → Envelope → Normalizer → Receipt Chain.
- SchemaWiring: Tool-Schema Optimization Bridge.
- DedupWiring: Content-Dedup Global Instance.
- In forward.rs und post_dispatch.rs verdrahtet.
- KERNEL_TEST_LOCK für Race-Condition-freie Tests.
- 346 Kernel-Tests.

### R27 (Production Activation)
- CtxReadDedup: Dedup Bridge für ctx_read Hot-Path (try_dedup, should_dedup, on_file_write).
- ListToolsOpt: Schema Optimization Bridge für list_tools (optimize_descriptions).
- KernelApi: HTTP Endpoints /v1/kernel/{dashboard,etpao,config,evidence,reset}.
- ConfigBridge: Kernel Config aus config.toml + ENV + Runtime.
- ProductionE2E: 8 E2E-Integrations-Tests.
- HTTP Routes live verdrahtet.
- 374 Kernel-Tests, 0 Clippy Errors.


### R28 (Last Mile — Hot-Path Live-Wiring)
- DedupHook: In ctx_read/mod.rs verdrahtet → Re-Reads liefern 15-Token Stub.
- SchemaHook: In server_handler.rs list_tools verdrahtet → Budget-aware Description Compression.
- Startup: In serve() + serve_ipc() verdrahtet → Kernel auto-konfiguriert beim Start.
- EvidenceHook: Unified Evidence Recording für alle Tool-Calls.
- SmokeTest: 6 Integration-Tests für echtes Hot-Path-Wiring.
- 384 Kernel-Tests, 0 Clippy Errors.
- **MEILENSTEIN: Kernel ist nun LIVE — spart real Tokens in Production.**

### R29 (Kernel Hardening — Evidence, Adaptive, Search Dedup, Health)
- EvidenceWiring: Dispatch-Level Evidence für Tool- und Proxy-Calls in post_dispatch verdrahtet.
- AdaptiveBridge: Bounce-Rate → Compression-Advice (Reduce/Maintain/Increase) über Kernel Config.
- SearchKernel: Query-Dedup-Detection + Evidence-Recording für ctx_search mit is_enabled() Gating.
- Health: Aggregiertes Kernel-Subsystem Health-Report (Dedup, Schema, Evidence, Config).
- IntegrationE2E: 6 E2E Conformance-Tests (Full Lifecycle, Evidence Sources, Adaptive, Health, Disabled, Search Dedup).
- 403 Kernel-Tests, 0 Clippy Errors.
- **MEILENSTEIN: Kernel vollständig observierbar + selbstregulierend.**

### R30 (Feedback Loop Closure — Search + Adaptive + Health API + Response Evidence)
- SearchHook: ctx_search (regex/semantic/symbol/batch) → search_kernel Evidence + Dedup.
- AdaptiveHook: bounce_tracker → adaptive_bridge Compression-Advice in ctx_read.
- HealthApi: /v1/kernel/health HTTP-Endpoint + Enhanced Dashboard mit allen Subsystemen.
- ResponseEvidence: Output-Token-Tracking pro Tool-Call in post_dispatch.
- FeedbackE2E: 6 E2E-Tests (Full Loop, Adaptive, Response, Dashboard, Disabled, Search Repeat).
- macOS Sequoia Codesign-Fix in dev-install.sh.
- 419 Kernel-Tests, 0 Clippy Errors.
- **MEILENSTEIN: Feedback-Regelkreis geschlossen — Kernel beobachtet, adaptiert und verbessert sich selbstständig.**

## Premium Production Readiness (E-Phasen, ab 2026-07-27)

### E2 (ETPAO Runtime Baseline — 4 Agents)
- `savings_ledger/etpao.rs`: RuntimeEtpao-Berechnung aus echten Ledger-Events.
- `telemetry.rs`: ObservedEfficiency Export (Cache Hit Rate, Request Count).
- `ctx_gain.rs`: ETPAO-Section im Dashboard mit Live-Daten.
- `efficiency_analyzer.rs`: 5 E2E-Testszenarien.
- 9095 Tests, 0 Clippy Warnings.
- **MEILENSTEIN: ETPAO-Metriken basieren auf echten Runtime-Daten statt Fixture-Nullen.**

### E3 (Multi-Layer Cache Pipeline — 6 Agents)
- **Root Cause Fix**: `telemetry.record_cache()` wurde in Produktion nie aufgerufen → ~0.7% Hit Rate.
- SessionCache + ContentCache hits → zentrale Telemetrie verdrahtet.
- ResponseCache aktiviert für deterministische Tool-Calls.
- Cache Warming Modul, Multi-Layer Dashboard (Session/Content/Response).
- E2E Pipeline-Tests (`cache/pipeline_tests.rs`).
- 879 neue LOC, 9137 Tests, 0 Clippy Warnings.
- **MEILENSTEIN: Alle 3 Cache-Layer fließen in die zentrale Metrik — echte Hit Rates sichtbar.**

### E4 (A2A Transport Hardening — 5 Agents)
- `a2a/remote_transport.rs` (342 LOC): HTTP Transport mit Retry, Timeout, Auth.
- `a2a/health.rs` (145 LOC): Transport Health Probes (Ready/Degraded/Unavailable).
- `a2a/relay.rs` (149 LOC): Multi-Hop Relay Chain + Cycle-Detection + Max-Hop-Limit.
- `a2a/budget_cascade.rs` (201 LOC): Token Budget Parent→Child Cascade mit Lineage.
- `a2a/telemetry.rs` (139 LOC): Transport Delivery Metrics.
- 976 neue LOC, 9147 Tests, 0 Clippy Warnings.
- **MEILENSTEIN: A2A-Subsystem hat Remote Transport, Health, Relay, Budget und Telemetrie.**
