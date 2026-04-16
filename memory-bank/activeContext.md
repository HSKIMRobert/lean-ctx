# Active Context

## Aktueller Stand (April 2026)
- Branch `epic/context-runtime-middle-layer`: Alle 3 Phasen implementiert.
- Website Deploy-Worktree: manifest-driven (42 Tools, 10 Read Modes), i18n aktualisiert, CSS-Fix committed.

## Was zuletzt gebaut wurde
- **Phase 1**: SSOT Manifest Generator + CI Gate, 42 Tools verdrahtet, 10 Read Modes aligned
- **Phase 2**: Workflow State Machine + Tool Gatekeeper + Evidence Store + `ctx_workflow` Tool
- **Phase 3**: `ContextEngine` Library API + `lean-ctx serve` HTTP Server Mode + Rate Limiting + Timeouts
- **Zero-Config Setup**: Editor Registry + Setup v2 (`--non-interactive/--yes/--fix/--json`) + SetupReport
- **Autopilot CLI**: `doctor --fix`, `bootstrap`, `status` (inkl. JSON Reports) + CI Smoke Test
- **Rules Injection**: v9 Templates (10 Read Modes inkl. `auto`, plus `ctx_workflow/ctx_cost/ctx_heatmap`)
- **Memory Runtime v1 (Autopilot, token-first)**:
  - Output Contracts + Budgets (deterministisch, no-spam)
  - `token-report` CLI + CI smoke
  - Archive-only Memory Lifecycle + Adaptive Forgetting + Retrieval Signals
  - Deterministische Salience Scoring
  - Consolidation Engine (background)
  - Dual-Process Retrieval mit Archive-Rehydration
  - Prospective Memory Reminders (task-gated) in `ctx_overview` + `ctx_preload`

## Offene Punkte
- GitLab Issues/Epics finalisieren (Status/AC/DoD)
- Finaler Cleanup/Commit-Strategie über beide Worktrees
- Memory Bank wurde bereinigt (Legacy-Duplikate entfernt)
 - Memory Runtime: alle Phasen (A–I) umgesetzt; finaler Review/Commit fehlt

## Aktive Entscheidungen
- **Kein Push zu GitHub** bis alles final reviewed ist
- **Deploy-Branch** enthält Website-Änderungen (separater Worktree)
- **http-server Feature** ist default ON
- **Version hardcoded** in 7+ Stellen (siehe release-checklist.md)
- **Node.js**: `/opt/homebrew/opt/node@22/bin` für Astro Builds
