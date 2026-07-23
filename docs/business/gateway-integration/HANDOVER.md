# HANDOVER — LeanCTX Token-Control-Platform

> **INTERNAL — Betriebsgeheimnis.**
> Stand: 2026-07-23 (nach R30 + Cockpit C1-C3)

## Schnelleinstieg

LeanCTX ist eine **kundeneigene Token-Control-Platform** fuer AI-Agenten.
Sie beobachtet, kontrolliert und optimiert den vollstaendigen Tokenstrom
eines Unternehmens — lokal, ohne zentrale Kundendaten.

### Repository

| Remote | URL | Inhalt |
|---|---|---|
| GitHub (OSS) | `github.com/yvgude/lean-ctx` | Engine + MCP + Proxy + CLI |
| GitLab (privat) | `gitlab.pounce.ch/root/lean-ctx` | OSS + Business Docs + Memory Bank |
| Enterprise | `lean-ctx-enterprise` | AI Value Gate (P10) — noch leer |

### Architektur

```text
Agents/Apps -> Context Broker/Control Kernel -> Customer-owned Token Data Plane
           -> OCLA Control -> Unified Ledger -> AI Value Gate
```

- **14 OCLA Traits** (Rust + Wire Contract)
- **Context Control Kernel** mit Plan/Receipt/Outcome-Loop
- **3 Betriebsplanes**: Data, Control, Value/Evidence
- **5 Schichten**: Engine, OCLA Contract, Unified Ledger, Interception Points, AI Value Gate

### Technologie

- **Sprache:** Rust (Proxy, CLI, MCP Server, Kernel) + JavaScript (Dashboard)
- **Build:** `cargo build --release` im `rust/` Verzeichnis
- **Tests:** `cargo test --lib` (9000+ Tests)
- **Install:** `lean-ctx dev-install` (Stop -> Build -> Install -> Restart)
- **Prozess:** LaunchAgent mit KeepAlive (macOS)

## Aktueller Status

### Was fertig ist (Engineering Milestone)
- **P0-P9, P11**: 100% auf `main` (30 Agent-Runden R1-R30)
- **14/14 Traits** produktiv verdrahtet
- **Context Kernel**: LIVE in allen Hot-Paths, 419+ Tests
- **Waves W0-W10**: Abgeschlossen
- **Cockpit**: 5 Areas + Kernel API + CSS Cleanup

### Was NICHT fertig ist
- **P10 AI Value Gate**: Nicht gestartet (privates Repo)
- **GA-Zertifizierung**: Gates G0-G10 nicht formal durchlaufen
- **Requirements Matrix**: 102/114 Partial
- **A2A Remote**: Nur hermetic Tests, kein Multi-Hop E2E
- **Token Efficiency**: Aspirational targets, nicht production-validated
- **Repository Hardening**: Branch Protection, SBOM, Signing ausstehend

## Dokumentenbaum (Lesereihenfolge)

1. `token-control-platform.md` — Was wir bauen und warum
2. `spec.md` — Normative Acceptance Criteria
3. `master-plan.md` — Phasen, Tracks, Dependencies
4. `premium-transformation-program.md` — Vollstaendiges Programm W0-W10
5. `ultimate-token-efficiency.md` — ETPAO und Effizienz-Targets
6. `holistic-context-intelligence.md` — Kernel-Architektur
7. `decisions.md` — ADR-001 bis ADR-022
8. `requirements-traceability.md` — **Autoritaere Statusquelle**
9. `OCLA-UMBAU-ZIEL.md` — Engineering Fortschritts-Tracker
10. `execution-playbook.md` — Delivery-Regeln
11. `repository-delivery-boundary.md` — Repo-Topologie
12. `tasks.md` — GitLab-Tickets

## Arbeitsregeln

1. **Status**: Nur `requirements-traceability.md` ist autoritaer fuer Completion
2. **Commits**: Pre-commit Hook erfordert `cargo fmt + clippy --all-features -D warnings`
3. **Push**: Pre-push Hook erfordert Whitespace-Check + Clippy + Docs + Registry
4. **GitHub**: Nur OSS-Code, keine Business-Docs oder Memory-Bank
5. **GitLab**: Alles (Code + Docs + Memory-Bank)
6. **LOC-Gate**: Keine Datei ueber 1500 Zeilen
7. **Prozess-Stop**: `lean-ctx stop` VOR jedem Build (LaunchAgent!)
8. **Branch**: `main` -> GitHub + GitLab; `deploy` -> NUR GitLab
