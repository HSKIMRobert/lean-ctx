# OCLA Gateway-Integration — Handover

Stand: `main` bei `15e1a0f85` (R5-Integration: P4-Abschluss, P5-Härtung, P7 Wire, P8/P9 Chiptuner).

## Definition des Status

„Verdrahtet" bedeutet hier: Der Builtin wird über
`OclaRegistry::global()` aus einem produktiven Laufzeitpfad aufgerufen.
„Gehärtet" bezeichnet einen verdrahteten Pfad mit expliziten Boundary-,
Fail-closed- oder TOCTOU-Sicherungen. „Offen" bezeichnet einen realen und
getesteten Builtin ohne produktiven Aufrufer; das ist kein Stub.

## Builtin-Inventar

| Builtin | Status | Aktueller Produktionspfad / Lücke |
| --- | --- | --- |
| `AgentGateway` | Verdrahtet | `tools/ctx_agent` nutzt Relay und Agent-Bus-Routing mit Envelope-Validierung. |
| `CompressionProvider` | Verdrahtet, gehärtet | Aggressive `ctx_read`-Kompression; ContentPort mit PathJail, BLAKE3-Referenz und fail-closed Gates. |
| `ConfigTuner` | Verdrahtet | Adaptive-Mode-Policy erzeugt deterministische Vorschläge mit Approval-Semantik. |
| `ConnectorScheduler` | Verdrahtet | Provider-Pipeline wählt verfügbare Connectoren bzw. Active-Inference-Fallback. |
| `EfficiencyAnalyzer` | Verdrahtet | `core/tool_lifecycle` berechnet Read-Density und ETPAO über den OCLA-Trait. |
| `ExperimentRunner` | Verdrahtet | Routing-Evaluation liefert deterministische Outcome- und Rollback-Referenzen. |
| `IntentClassifier` | Verdrahtet (R5) | `proxy/forward.rs` ruft `classify_intent()` im Pre-Forward-Pfad auf. |
| `MetricsExporter` | Verdrahtet | `tools/server_metrics` exportiert pro MCP-Call ein begrenztes lokales Batch (`#1093`). |
| `ModelRouter` | Verdrahtet, erweitert (R5) | Intent-aware Routing + Ledger-History-basierte Entscheidungen + A/B via ExperimentRunner. |
| `ObservationHook` | Verdrahtet | `tools/server_metrics` projiziert jeden MCP-Tool-Call als Observation. |
| `OutcomeTracker` | Verdrahtet | `tools/server_metrics` schreibt Accepted-/Quality-Ergebnis nach jedem MCP-Call. |
| `ResponseOptimizer` | Verdrahtet (R5) | OCLA-Trait-Pfad für Response-Optimierung; Output-Token-Messung im Ledger; Similarity-basierte Dedup. |
| `SavingsLedger` | Verdrahtet | OCLA-Evidence wird in den verifizierten Core-Ledger projiziert; Unified Ledger Dual-Write aktiv. |
| `UsageSink` | Verdrahtet | `proxy/usage_meter` projiziert den finalisierten Provider-Turn in den OCLA-Sink. |

Damit sind **14/14 Builtins** produktiv adoptiert (davon 1 zusätzlich gehärtet);
das entspricht **100 % Trait-Adoption**.

## Runde-5-Deliverables (auf `main`)

### Track A: P4-Abschluss + P5-Härtung
- `07e1dbfea`: IntentClassifier im Proxy-Lifecycle adoptiert
- `3b60c4946`: ResponseOptimizer OCLA-Adoption mit Output-Token-Messung
- `49b7a56dd`: P5 Dual-Write (UnifiedSavingsEventV2 parallel zum Legacy-Ledger)
- `e12f8458b`: Approval/Settlement-Workflow (`approve_event`, `settle_event`, `query_pending_approval`)

### Track B: P7 Wire Contract
- `122fa1b85`: REST API Server (`/ocla/v1/health`, `/capabilities`, `/envelope`, `/ledger/summary`)
- `c1ebfa714`: OpenAPI 3.1 Spec mit Contract-Drift-Snapshot-Test
- `9b713e21a`: Wire Streaming Semantik (`StreamFrame`, Cancellation, Deadline, Backpressure)
- `85cbbf0ef`: Contract Golden Suite (Envelope, Schema, External Consumer Tests)

### Track C: P8/P9 Chiptuner
- `dc7bbaebb`: P8 Model Router — Intent-basiertes Routing + Ledger-History + `intent_aware_effort()`
- `941d44172`: P9 Response Optimizer — Similarity-basierte Dedup + Token-Measurement

## Gemergter Stand auf `main` (kumuliert)

- R1–R3: P0 Done, P1 (14 Traits), P2 (OclaBus), P3 (15 Builtins), P4 (12/14 Adoption)
- R4: P5 Schema (19 Felder, v5 Hash, Evidence Classification, Attribution IDs, Query APIs, CLI)
- R5: P4 100%, P5 Dual-Write + Approval, P7 Wire REST + OpenAPI + Streaming + Contract Suite, P8 Intent-Routing, P9 Response-Optimization

## Phase-Status nach R5

| Phase | Status | Fortschritt |
| --- | --- | --- |
| P0 IST-Hygiene | DONE | 100% |
| P1 OCLA Contract | DONE | 100% |
| P2 OclaBus | DONE | 100% |
| P3 Built-ins | DONE | 100% |
| P4 Trait-Adoption | DONE | 14/14 = 100% |
| P5 Unified Ledger | PARTIAL | Schema + Dual-Write + Approval — Budget Cascade offen |
| P6 Binary-Sep | DEFERRED | — |
| P7 Wire Contract | PARTIAL | REST API + OpenAPI + Streaming + Contract Suite — SDKs fehlen |
| P8 Model Router | PARTIAL | Intent-Routing + Ledger-History — A/B-Benchmarks fehlen |
| P9 Response Opt. | PARTIAL | Similarity-Dedup + Token-Messung — Response-Cache vollständig verdrahtet |
| P10 AI Value Gate | COMMERCIAL | Nicht im OSS-Repo |
| P11 Deploy+A2A | TRIGGERED | Wartet auf P10 |

## Nächste Schritte

1. P7 Vervollständigung: Partner-SDKs (Python, TypeScript, Go, Java), gRPC-Contract,
   Sidecar-Deployment-Profile.
2. P5 Budget Cascade: Reservation/Consumption/Deallocation über Agent-Chain.
3. P8/P9 Benchmarks: A/B-Testing-Kampagnen mit ExperimentRunner, Quality Gates.
4. P10 Vorbereitung: AI Value Gate im privaten `lean-ctx-enterprise` Repo.

## OSS/Private-Boundary-Audit

R5 hat keine neuen Enterprise-/SSO-/Commercial-Artefakte eingeführt.
`cloud-infra/` bleibt entfernt. Alle R5-Deliverables sind Apache-2.0 OSS.

## A2A Operational Hardening (ADR-022)

Stand: 2026-07-21. 6 identifizierte Gaps:

| # | Gap | Status nach R5 |
|---|---|---|
| 1 | Budget Cascade | P5 Approval/Settlement-Workflow gestartet; Cascade offen |
| 2 | Reconciliation Loop | Offen (P11) |
| 3 | CoW Context Capsules | Offen (P11) |
| 4 | Dead Letter Queue | Offen (P11) |
| 5 | Agent Health Surface | P7 REST API `/ocla/v1/health` + `/capabilities` liefert Grundlage |
| 6 | Distributed Tracing | Offen (P5/P11) |
