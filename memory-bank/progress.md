# Progress

## Fertig (Phase 1–3)
- **SSOT Manifest**: Generator + CI Gate, `website/generated/mcp-tools.json`
- **42 Tools / 10 Read Modes**: Runtime + Website aligned
- **Rails**: Workflow State Machine + Gatekeeper + Evidence + `ctx_workflow`
- **Observability**: `ctx_cost`, `ctx_heatmap` wired + default-on (local-first)
- **Library API**: `lean_ctx::engine::ContextEngine` (call_tool_text, call_tool_value, call_tool_result, manifest)
- **HTTP Server**: `lean-ctx serve` mit Streamable HTTP + REST Endpoints
- **Security Defaults**: Loopback bind, Host-header validation, Bearer Auth, Rate Limiting, Concurrency Limits, Request Timeouts
- **Website**: Manifest-driven counts (i18n Placeholders), CSS Fix (mobile sidebar chevron)
- **Zero-Config Setup**: Editor Registry + Setup v2 (`--non-interactive/--yes/--fix/--json`) + SetupReport JSON
- **Autopilot CLI**: `doctor --fix`, `bootstrap`, `status` (inkl. JSON) + CI Smoke Test (`setup_ci_smoke.rs`)
- **CI**: GitHub Actions Test-Matrix (Ubuntu/macOS/Windows)
- **Tests**: 770+ lib tests + Integration/Smoke Tests, alle grün
- **Memory Runtime v1 (Autopilot, token-first)**:
  - Budgets/Output Contracts + deterministische Knowledge-Ausgaben
  - `token-report` (JSON + human) + CI smoke
  - Archive-only Lifecycle (keine Hard-Deletes), Adaptive Forgetting + Retrieval Signals
  - Dual-Process Retrieval (System 1/2) + Archive Rehydration
  - Consolidation Engine (background, budgeted)
  - Prospective Reminders (task-gated) in `ctx_overview` + `ctx_preload`

## Ausstehend (Prozess)
- GitLab Issues/Epics finalisieren
- Finaler Cleanup/Commit über beide Worktrees

## Bekannte Einschränkungen
- Version hardcoded in 7+ Stellen

## Distribution
- [x] crates.io
- [x] Homebrew tap (yvgude/lean-ctx)
- [x] AUR (lean-ctx + lean-ctx-bin)
- [x] GitHub Releases mit CI binaries (5 Targets)
- [x] Cross-platform: macOS, Linux, Windows
