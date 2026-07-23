# Execution Backlog — Token-Control-Platform

> **INTERNAL — Betriebsgeheimnis.** · Stand 2026-07-23 (aktualisiert nach R30)
> GitLab Epic #1196 und Issues sind Execution-Tracker. Dieses Dokument definiert
> die erforderliche Struktur; Ticketstatus muss vor Start gegen GitLab geprueft
> werden. Programm: `premium-transformation-program.md`. Effizienz:
> `ultimate-token-efficiency.md`.

## 1. Status Baseline (2026-07-23)

| Element | Verifizierter Worktree-Status |
|---|---|
| P0 IST-Hygiene | **DONE** |
| P1-P9, P11 | **DONE** auf `main` (30 Agent-Runden R1-R30) |
| P10 AI Value Gate | OFFEN (lean-ctx-enterprise) |
| `rust/src/core/ocla/` | existiert, 14 Traits live |
| Canonical Token Envelope | existiert (`token_envelope.rs`) |
| OCLA Wire Contract/Contract Suite | existiert (REST/gRPC/OpenAPI + 3 SDKs) |
| Context Kernel | LIVE, 419+ Tests, Feedback-Loop geschlossen |
| Enterprise GA | nicht erreicht (Requirements Matrix: 102/114 Partial) |

## 2. OCLA Work-Packages

| Package | GitLab | Kernlieferung | Programm-Wave | Status |
|---|---:|---|---|---|
| P0 | #1197 | IST-Hygiene | W0 | **DONE** |
| P1 | #1201 | 14 Traits, Types, Errors, Discovery, Envelope-Basis | W1 | **DONE** (R1-R4) |
| P2 | #1202 | bounded OclaBus + Event Schema | W2 | **DONE** (R1-R4) |
| P3 | #1203 | lokale Built-ins | W2 | **DONE** (R1-R4) |
| P4 | #1207 | Trait-Adoption + Benchmark Gates | W7 | **DONE** (R5, 14/14) |
| P5 | #1216 | Unified Ledger + Evidence Semantik | W2 | **DONE** (R5-R10) |
| P6 | #1220 | physische Crate-/Binary-Separation | W6/W8 | **DEFERRED** (absorbiert) |
| P7 | #1221 | Wire Contract, Schemas, SDKs, Contract Suite | W6 | **DONE** (R5-R10) |
| P8 | #1225 | Model Router | W7 | **DONE** (R5-R10) |
| P9 | #1226 | Response Optimizer | W7 | **DONE** (R5-R10) |
| P10 | #1227 | AI Value Gate v0 | W9 | **OFFEN** (lean-ctx-enterprise) |
| P11 | #1228 | AgentGateway/Packaging | W7/W8 | **DONE** (R7-R10) |

`Deferred` bedeutet nicht „optional". P6 startet am Extraction Gate; P11 am
Agent-/Enterprise-Adoption-Gate. Beide sind vor GA neu zu entscheiden.

## 3. Zusätzliche Programm-Epics

P0–P11 reichen für die Gesamttransformation nicht. Folgende Epics müssen im
Execution-Tracker vorhanden sein:

### EP-A — Reality & Baseline (W0)

- request-path-inventory;
- capability-maturity-audit;
- protocol/provider coverage matrix;
- runtime-belegte IDE Coverage Classes und Client Efficiency Profiles;
- ETPAO-Baseline für Input/Reasoning/Output/Schema/Cache/Retry/Re-Read/A2A;
- latency/throughput/quality baseline;
- golden traffic corpus;
- SSOT/status cleanup;
- risk register and owners.

**Exit:** G0 Evidence Pack.

### EP-B — Canonical Envelope & Protocol Semantics (W1/W4)

- canonical request/message/content/tool/usage/error types;
- OpenAI Chat + Responses adapters;
- Anthropic Messages adapter;
- Gemini/OpenAI-compatible adapters;
- streaming/tool-call golden traces;
- unsupported/lossy capability reporting;
- provider extension policy.

**Exit:** G1/G4 Evidence Packs.

### EP-C — Compression Quality Lab (W3)

- multi-format corpus;
- lossless/controlled-lossy classification;
- tokenizer/model-family calibration;
- semantic fidelity and task-success benchmark;
- instruction/tool-schema overhead and stable-prefix benchmark;
- reasoning/output/ETPAO and duplicate-context benchmark;
- multi-agent single-vs-parallel outcome-parity benchmark;
- adversarial compression safety corpus;
- regression budget CI;
- recovery/fallback tests.

**Exit:** G3 Evidence Pack.

### EP-D — Provider Fabric & Reliability (W4/W8)

- provider/model/region registry;
- credentials by reference and rotation;
- retry/circuit breaker/health/failover;
- rate limiting and backpressure;
- shape translation for stream/tools/errors;
- load/soak/chaos tests;
- explicit non-interceptable traffic matrix.
- client-adaptive Context Broker, minimal tool profiles and Handle/Delta path.

**Exit:** G4/G8 Evidence Packs.

### EP-E — Identity, Policy & Fleet Control (W5)

- human/service/agent/workload identity;
- org/team/project/cost-center attribution;
- PDP/PEP and deterministic policy precedence;
- model/provider/region/sensitivity/egress rules;
- budget/quota/rate enforcement;
- signed policy bundles, shadow/staged/rollback/break-glass;
- fleet health and config drift.

**Exit:** G5 Evidence Pack.

### EP-F — SDK & Certification Ecosystem (W6)

- OpenAPI/Protobuf drift gate;
- Rust, TypeScript and Python SDKs first;
- Java, .NET and Go SDKs after core stability;
- reference sidecar and external gateway;
- contract conformance suite;
- compatibility/deprecation/release policy;
- external consumer project and certification.

**Exit:** G6 Evidence Pack.

### EP-G — Enterprise Security & Operations (W8)

- threat model and abuse cases;
- TLS/mTLS, secrets, rotation, least privilege;
- tenant isolation and enterprise authz;
- HA, graceful drain, zero-downtime upgrade, rollback;
- SLOs, metrics, traces, SIEM and alerting;
- Helm/Terraform/air-gap;
- backup/restore/DR;
- retention/delete/residency/legal hold;
- SBOM, signing, provenance and vulnerability response.

**Exit:** G8 / Production Readiness Review.

### EP-H — Commercial Operations (W9)

- AI Value Gate role-based views;
- baseline calibration workflow;
- quality/evidence/approval/dispute/settlement state machine;
- customer-approved export and manual air-gap flow;
- license-expiry/no-cloud failure tests;
- setup/subscription/support/value-share schedules;
- invoice provenance and correction E2E;
- support/LTS/SLA policy.

**Exit:** G9 Evidence Pack.

### EP-I — Pilot & GA (W10)

- customer traffic/coverage discovery;
- baseline + shadow acceptance;
- limited control rollout;
- quality-bounded optimization rollout;
- approved automation;
- migration/rollback/incident/exit plan;
- admin/developer/security/finance training;
- second clean-room customer overlay;
- GA documentation and independent runbook execution.

**Exit:** G10 Evidence Pack.

### EP-J — Repository, Supply Chain & Delivery Boundary (W0/W8/W10)

- aktive Credentials aus lokaler Betriebsreferenz rotieren und Secret Store
  als SSOT festlegen;
- GitHub als kanonischen OSS Source und GitLab als read-only Mirror etablieren;
- Force/Direct Push deaktivieren, Required Reviews/CI und CODEOWNERS aktivieren;
- `lean-ctx-cloud` CI, Plane Guard, SBOM und signed-image Pipeline aufbauen;
- öffentliche History auf Secrets/IP prüfen und Rewrite-/No-Rewrite-Entscheid
  dokumentieren;
- Website aus dem OSS-Git-Objektbestand in privates Repository migrieren;
- private versionierte Internal-Docs-Quelle mit Backup/Restore und Secret Scan;
- Contract-/Dependency-Direction-Gates zwischen OSS, Cloud und Enterprise;
- immutable OCI Digests, Deployment Manifest und Promotion-Pipeline;
- Customer-Overlay Policy: kein Code, keine Klartext-Secrets, nur Pins/Values;
- zweites Overlay deployen und digestgebunden zurückrollen.

**Exit:** RG-01..RG-12 belegt; Repository/Delivery-Anteil von G0, G8 und G10 grün.

### EP-K — Holistic Context Intelligence (W0–W10)

- Store-/Identity-/Handle-/Cache-/Planner-/Feedback-Callgraph und Reality Map;
- `ContextObjectV1`, `ContentRef`, Candidate/Materializer und Memory-Horizon Types;
- `ContextPlanV1` und `ContextReceiptV1` mit deterministischer Lineage;
- Candidate Fabric für Session, Knowledge, Cache, Graph/Search, Episodes,
  Procedures, Prospective Memory, Provider und Agent Evidence;
- Context Field/Compiler als gemeinsamer Runtime-Service statt nur Tool-Surface;
- Shadow-Adoption in MCP Read/Search/Compose/Shell, Proxy und AgentGateway;
- gemeinsame Invalidation für Source, Policy, Delete, Retention und Quality;
- append-aware Stream Controller für IDE-Terminals, Logs und Watcher mit
  Generation, Cursor, Delta, Rotation und Recovery;
- `CacheReceiptV1` und Cache Broker für L0 Delivery, L1 View, L2 Workspace,
  L3 Provider Prefix und L4 Knowledge/Capsule;
- Request-/Session-/Agent-/ContentRef-Korrelation und getrennte
  Payload-/Control-/Cache-/Ledger-Telemetrie ohne Doppelzählung;
- Outcome Learning über Context, Compression, Routing, Output und Agenten;
- ContextCapsule/Deltas/Leases/Work-Graph-Fusion ohne Prompt-Anhängen;
- ETPAO/Recall/Freshness/Duplicate/First-Pass Quality Lab und Rollback.

**Exit:** HC-01..HC-18 belegt; jeder kontrollierte Hot Path erzeugt Plan/Receipt
oder besitzt eine explizite Coverage-Ausnahme.

## 4. Immediate Critical Path

```text
T0  EP-J Containment: Secrets, Branch Protection, History Audit, Cloud CI
T1  Reconcile GitLab status and create missing EP-A..EP-K trackers
T2  W0 reality baseline + live/cache/stream audit + golden corpus + deployed-digest inventory
T3  P1 OCLA + Envelope + ContextObject/Plan/Receipt contracts
T4  P2 event bus and P5 evidence schema in parallel with adapter fixtures
T5  W3 compression + append-stream quality gates
T6  W4 Data Plane + Context Kernel + L0-L4 Cache Broker shadow adoption
T7  P7 Wire Contract + first 3 SDKs + external consumer
T8  W5 controlled pilot path
T9  W7 optimization dimensions
T10 W8 enterprise hardening + W9 commercial readiness
T11 W10 lighthouse → second deployment → GA
```

P8/P9 dürfen nicht vor W3 Quality Baseline als produktiv gelten. AI Value Gate
darf nicht vor P5 Evidence Semantik abrechnungsfähige Savings anzeigen.

## 5. Required Ticket Fields

Jedes Ticket enthält:

- Requirement IDs aus `requirements-traceability.md`;
- Wave, Gate und OCLA-Package;
- Repo und OSS/Commercial Bucket;
- genaue Contract-/Runtime-/Data-Change-Points;
- Abhängigkeiten und Compatibility-Auswirkung;
- Security/Privacy/Failure Modes;
- Tests, Benchmarks und negative cases;
- Rollout/flag/rollback/observability;
- Branch-/CODEOWNER-/Plane-Guard und, falls deploybar, Contract-/Image-Digest;
- ausführbare Acceptance Evidence;
- Docs/ADRs/SDKs, die synchronisiert werden.

## 6. Global Definition of Done

- `cargo fmt`, `cargo test --lib`, relevante Integrations-/E2E-Tests;
- `cargo clippy -- -W clippy::all` ohne Warnungen;
- deterministische Outputs und Schema-/Contract-Drift Gates;
- keine verbotene OCLA→Runtime-Abhängigkeit;
- Hot-Path innerhalb vereinbartem Latenz-/Speicherbudget;
- Local-Free, Customer-Owned und no-double-count Invarianten grün;
- Repo-/Dependency-Direction-Guard grün; keine Secret-/Commercial-Leaks;
- deploybare Artefakte besitzen SBOM, Provenance, Signatur und immutable Digest;
- rollback/recovery und negative Security Tests;
- Requirement Matrix verlinkt echte Evidence;
- Handover, ADRs und Status aktualisiert.
- bei Context-Änderungen: Candidate-/Plan-/Receipt-/Invalidation-Auswirkung und
  HC-Requirement explizit dokumentiert.

## 7. Program Completion

Nicht P10, sondern G10 beendet das Programm. Voraussetzung: alle Pflicht-
Requirements belegt, externe OCLA-Integration zertifiziert, Enterprise PRR
bestanden und zweites kundenähnliches Deployment ohne Code-Fork reproduziert.
