# Active Context

Stand: 2026-07-23T14:30+02:00

## Aktueller Fokus

Cockpit Architecture Audit abgeschlossen. Premium-Consolidation-Roadmap erstellt.

## Letzte Änderungen

### R31-R33 (Provider Pipeline)
- ProviderKind +Bedrock/Azure, provider_parity, envelope_bridge, usage_parity
- Response-Path Wiring: usage_meter::record → envelope → kernel pipeline
- 478 Kernel-Tests, 0 Clippy

### Cockpit Audit (Findings)
- **22 Views → 7** geplant (Context, Knowledge, Savings, System, Code, Activity, Home)
- **518KB JS → ~200KB** Ziel
- **10 kritische Findings** dokumentiert (F1-F10)
- **8 Umsetzungs-Phasen** definiert (C1-C8)
- Hauptprobleme: Massive Duplikation, kein Kernel-API Consumer, 79KB Graph-View, defekter Explorer

## Architektur-Status

### OCLA: P0-P9, P11 = 100%
### Context Kernel: 32 Runden, 478+ Tests
### Provider Pipeline: detect → envelope → bridge → stats → dashboard ✓
### Cockpit: Audit done, Consolidation pending (C1-C8)

## Nächste Schritte

1. **Phase C1**: Context Consolidation (context + commander + compression → 3 Tabs)
2. **Phase C2**: Savings & ROI Consolidation (+ Kernel Provider Stats)
3. **Phase C4**: System Health + Kernel-API Integration
4. Dann C3, C5-C8

## Dokumentation

- `memory-bank/cockpit-audit.md` — Vollständiger Audit-Report
- `memory-bank/progress.md` — R31-R33 Fortschritt
