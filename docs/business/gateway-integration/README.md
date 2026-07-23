# LeanCTX Token-Control-Platform — Internal Architecture

> **INTERNAL — Betriebsgeheimnis.** Dieses Verzeichnis ist via `.gitignore`
> geschuetzt und wird nicht oeffentlich gepusht.

LeanCTX wird von einer monolithischen Context-Runtime zur kundeneigenen
Enterprise Token-Control-Platform umgebaut. Der vollstaendige adressierbare
Tokenstrom eines Unternehmens kann lokal beobachtet, kontrolliert, optimiert
und wirtschaftlich attribuiert werden. Thinkery benoetigt dafuer keine zentralen
Kundendaten.

## Source of Truth

| Artefakt | Verantwortung |
|---|---|
| [token-control-platform.md](token-control-platform.md) | Produktarchitektur, Enterprise-Modell, OSS/Commercial, Datenhoheit |
| [ultimate-token-efficiency.md](ultimate-token-efficiency.md) | End-to-End-Effizienz fuer Input, Reasoning, Output, Cache, IDEs und Multi-Agent |
| [holistic-context-intelligence.md](holistic-context-intelligence.md) | Context Control Kernel, Memory-Horizonte, Candidate Fabric, Plan/Receipt und Outcome Learning |
| [../product-architecture.md](../product-architecture.md) | Commercial, Pricing Policy, Deployment und Naming |
| [premium-transformation-program.md](premium-transformation-program.md) | vollstaendiges Umbauprogramm, Waves W0-W10, Gates und Ressourcen |
| [master-plan.md](master-plan.md) | Phasen, Tracks, Dependencies und Deliverables |
| [spec.md](spec.md) | normative OCLA- und Plattform-Acceptance-Criteria |
| [requirements-traceability.md](requirements-traceability.md) | Requirement-Status und erforderliche Completion Evidence |
| [execution-playbook.md](execution-playbook.md) | Delivery-, Test-, Rollout- und Evidence-Regeln |
| [repository-delivery-boundary.md](repository-delivery-boundary.md) | GitHub/GitLab/Server, OSS/Commercial, Branching, Supply Chain und Deploy-Provenance |
| [runtime-token-intelligence-audit.md](runtime-token-intelligence-audit.md) | Live-Event-, Cache-, Terminal- und Agent-Brain-Istnachweis samt Exit Gates |
| [documentation-completion-audit.md](documentation-completion-audit.md) | formaler Nachweis der vollstaendigen Planungsgrundlage |
| [decisions.md](decisions.md) | Architekturentscheidungen im ADR-Format |
| [OCLA-UMBAU-ZIEL.md](OCLA-UMBAU-ZIEL.md) | Fortschritts-Tracker: Phasen, Waves, Kernel-Runden |
| [tasks.md](tasks.md) | GitLab-Tickets und Execution Gates |
| [zuhlke-partnership.md](zuhlke-partnership.md) | Partnerintegration und Pilotmodell |

## Architektur in einem Satz

```text
Agents/Apps -> Context Broker/Control Kernel -> Customer-owned Token Data Plane
           -> OCLA Control -> Unified Ledger -> AI Value Gate / Enterprise Subscription
```

OCLA definiert 14 kleine Capabilities und besitzt zwei Projektionen:

- Rust Traits fuer interne/in-process Integration;
- einen versionierten Wire Contract fuer Gateways, Sidecars und SDKs.

Der Data Plane ist Open Source, local-first und unabhaengig von Thinkery Cloud.
Thinkery monetarisiert Setup, Enterprise Subscription, Support und verifizierte
Netto-Savings.

## Aktueller Stand (2026-07-23, nach R30 + Cockpit C1-C3)

### OSS OCLA-Umbau: Engineering Milestone erreicht
- **P0-P9, P11: 100%** auf `main` (30 Agent-Runden R1-R30)
- **14/14 Traits** produktiv verdrahtet, alle Builtins live
- **Context Kernel**: LIVE in allen Hot-Paths (ctx_read, ctx_search, ctx_shell, ctx_compose, forward.rs, post_dispatch.rs)
- **419+ Kernel-Tests**, 0 Clippy Warnings
- **Waves W0-W10**: Alle abgeschlossen (Holistic Context Intelligence)
- **Feedback-Loop**: geschlossen (observe -> adapt -> improve)

### Noch offen
- **P10 AI Value Gate**: Privates Repo `lean-ctx-enterprise` (nicht Teil OSS)
- **Requirements Matrix**: 102/114 Partial — viele Targets sind aspirational, nicht production-validated
- **Token Efficiency Targets**: Aspirational (spec.md), nicht GA-zertifiziert
- **A2A**: Code existiert, aber nur hermetic same-process Tests; kein Remote/Multi-Hop E2E
- **Repository Hardening**: Branch Protection, SBOM, Production Signing noch ausstehend
- **GA-Zertifizierung**: Gates G0-G10 nicht formal durchlaufen

### Massgeblich fuer Status
Die **Requirements-&-Evidence-Matrix** (`requirements-traceability.md`) ist die
einzige autoritaere Statusquelle. `OCLA-UMBAU-ZIEL.md` ist der Engineering-
Fortschritts-Tracker. Status-Claims in aelteren Docs (plan.md, tasks.md etc.)
koennen veraltet sein.

## Naming

| Extern | Intern | Config |
|---|---|---|
| Token-Control-Platform | Gesamtsystem | — |
| OCLA Contract | `core/ocla/` | `[ocla]` |
| MCP Addon Gateway | `core/mcp_catalog/` | `[gateway]` |
| Org/Team Gateway | `gateway_server/` | `[gateway_server]` |
| AI Value Gate | kommerzielle Enterprise-Surface | eigene Config |

```text
token-control-platform.md -> product-architecture.md -> spec.md
-> repository-delivery-boundary.md -> premium-transformation-program.md
-> master-plan.md -> tasks -> code -> evidence
```
