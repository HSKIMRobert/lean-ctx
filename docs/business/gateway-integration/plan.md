# Operativer Kurzplan — LeanCTX Token-Control-Platform

> **INTERNAL — Stand 2026-07-23 (aktualisiert nach R30 + Cockpit C1-C3)**
> Gesamtplan: `premium-transformation-program.md` · OCLA: `master-plan.md` ·
> Effizienz: `ultimate-token-efficiency.md` · Repository/Delivery:
> `repository-delivery-boundary.md` · Context/Memory:
> `holistic-context-intelligence.md` · Status: `requirements-traceability.md`.

## Outcome

LeanCTX wird customer-owned Data, Control und Value/Evidence Plane fuer den
adressierbaren AI-Tokenstrom. OCLA verbindet interne Module, Partner-Gateways,
SDKs und externe Services ohne Abhaengigkeit vom Monolithen.

## Programm-Waves

| Wave | Outcome | Gate | Status |
|---|---|---|---|
| W0 | Repository Boundary + Context/Runtime Reality Baseline | G0 | **DONE** (R12) |
| W1 | OCLA + Envelope + Context Object/Plan/Receipt | G1 | **DONE** (R13) |
| W2 | Observe Bus + Unified Evidence | G2 | **DONE** (R15) |
| W3 | Compression Quality Lab | G3 | **DONE** (R14) |
| W4 | Data Plane + Provider Fabric + Context Control Kernel | G4 | **DONE** (R15) |
| W5 | Identity/Policy/Control Plane | G5 | **DONE** (R16) |
| W6 | Wire Contract/SDK/Certification | G6 | **DONE** (R16) |
| W7 | Input/Output/Routing/Agent Control | G7 | **DONE** (R16) |
| W8 | Enterprise Security/Reliability/Ops | G8 | **DONE** (R17) |
| W9 | AI Value Gate/Commercial Readiness | G9 | **DONE** (R18) |
| W10 | Lighthouse/Second Deployment/GA | G10 | **DONE** (R18) |

> **Hinweis:** Wave-Completion bezieht sich auf den *Engineering Milestone* (Code auf
> `main`). Die formale GA-Zertifizierung (Gates G0-G10 mit Evidence Packs) steht noch aus.
> Massgeblich ist die Requirements Matrix (`requirements-traceability.md`).

## OCLA Engineering Packages

| Phase | Status | Referenz |
|---|---|---|
| P0 IST-Hygiene | **DONE** | R1-R4 |
| P1 OCLA Contract | **DONE** | R1-R4 (14 Traits) |
| P2 OclaBus | **DONE** | R1-R4 |
| P3 Built-ins | **DONE** | R1-R4 (15 Builtins) |
| P4 Trait-Adoption | **DONE** | R5 (14/14) |
| P5 Unified Ledger | **DONE** | R5-R10 |
| P6 Binary-Sep | **DEFERRED** | Absorbiert in P0-P5 |
| P7 Wire + SDK | **DONE** | R5-R10 (REST/gRPC/OpenAPI + 3 SDKs) |
| P8 Model Router | **DONE** | R5-R10 |
| P9 Response Opt. | **DONE** | R5-R10 |
| P10 AI Value Gate | **OFFEN** | lean-ctx-enterprise (nicht OSS) |
| P11 Deployment + A2A | **DONE** | R7-R10 |

## Context Kernel (R13-R30)

- **419+ Tests**, 0 Clippy Warnings
- LIVE in allen Hot-Paths: ctx_read, ctx_search, ctx_shell, ctx_compose, forward.rs, post_dispatch.rs
- Feedback-Loop geschlossen: observe -> adapt -> improve
- Content-Dedup: 95-99% bei Unchanged, 150-Token Hard Cap

## Verbleibende Luecken (ehrlich)

### Engineering Gaps (Production Readiness)
1. **A2A Remote Transport** — nur hermetic same-process Tests, kein Multi-Hop E2E
2. **CEP Cache Hit Rate** — 0.7-1% beobachtet, Ziel >>50%
3. **Token Efficiency Targets** — aspirational, nicht production-validated
4. **Live Coverage Class Probing** — `runtime_path_complete=false`
5. **Requirements Matrix** — 102/114 Partial (viele Targets unvalidiert)

### Repository/Supply Chain
6. **Branch Protection** — GitHub required reviews/CI noch nicht enforced
7. **SBOM + Production Signing** — geplant, nicht implementiert
8. **Secret Rotation** — ausstehend
9. **Cloud CI Pipeline** — nicht konfiguriert

### Commercial
10. **P10 AI Value Gate** — nicht gestartet
11. **GA-Zertifizierung** — Gates G0-G10 nicht formal durchlaufen
12. **Second Deployment** — noch nicht geschehen

## Hard Gates

| Gate | Proof | Status |
|---|---|---|
| Contract | 14 Traits, Envelope, external consumer | ✅ Code done |
| Repository | protected main, OSS mirror, private repos | ⚠️ Partial (protection weak) |
| Supply Chain | Secret hygiene, SBOM, signature, provenance | ⚠️ Partial |
| Efficiency | ETPAO Input/Reasoning/Output/Schema/Cache/Retry/A2A | ⚠️ Aspiration only |
| Context Intelligence | Plan/Receipt/Invalidation in Hot Paths | ✅ Code done |
| Coverage | runtime Coverage Class + Clientprofil | ⚠️ Partial |
| Fidelity | golden streams/tools/errors + quality budget | ⚠️ Partial |
| Evidence | offline verify, quality-bound, no double count | ✅ Code done |
| Control | deterministic policy, shadow/staged/rollback | ✅ Code done |
| Reliability | HA/load/chaos/DR/SLO | ⚠️ Not tested |
| Commercial | invoice -> approved evidence | ❌ P10 not started |
| GA | second deployment without code fork | ❌ Not done |

## Ultimate Efficiency Sequence

```text
observe complete cost
-> stabilize prefix/schema
-> replace repeated payload with handles/deltas
-> minimize input and deterministic tool work
-> route smallest qualified model/effort
-> emit minimal typed output
-> coordinate agents via capsules, ownership and chain budgets
-> count only quality-preserving net reduction
```

## Customer Rollout

```text
OBSERVE -> MEASURE -> CONTROL -> OPTIMIZE -> AUTOMATE
```

Jede Stufe besitzt Owner, Population, Zeitraum, SLO/Quality Budget, Stop
Condition, Acceptance Evidence und Rollback. Automate ist nie Default.

## Completion

Die Dokumentationsgrundlage ist vorhanden. Das OSS OCLA Engineering Milestone
ist erreicht (P0-P11, W0-W10 auf `main`). Das Produktziel (GA) ist nicht
erreicht, solange die Requirement Matrix Partial-Pflichtzeilen enthaelt und
G0-G10 nicht mit Evidence Packs bestanden sind.
