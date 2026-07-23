
### R32 — Provider Live-Wiring + Envelope Bridge + Azure (5 Agents)
- UsageParity: provider_kind_from_label() + real_usage_to_envelope() + label_from_base_url()
- EnvelopeBridge: record_proxy_envelope() + record_mcp_envelope() + provider_stats() Pipeline
- ProviderMetricsE2E: 8 E2E-Tests für detect→envelope→record→stats Pipeline
- UsageAzure: is_azure_response() + normalize_azure_model() + parse_azure_usage()
- ProviderDisplay: format_provider_table() + provider_summary_oneliner() + provider_json()
- Wiring: Provider::from_label +Azure, DashboardReport +provider_distribution
- 478 Kernel-Tests + 17 Proxy-Tests, 0 Clippy Warnings
- **MEILENSTEIN: Provider-Pipeline vollständig verdrahtet — detect → envelope → stats → dashboard**

### R33 — Response-Path Wiring (manuell, kein Agent)
- Single-Choke-Point: usage_meter::record → real_usage_to_envelope → record_proxy_envelope
- Alle 3 Response-Pfade abgedeckt: HTTP Transport, Bedrock Streaming, WebSocket
- Provider-Label aus WireContext.provider extrahiert
- 478 Kernel-Tests, 0 Clippy
- **MEILENSTEIN: Jeder Provider-Response fliesst automatisch in die Kernel-Pipeline**

## Cockpit Architecture Audit & Premium-Integration

Stand: 2026-07-23 (aktualisiert)

### Architektur-Erkenntnis (C1 Tiefanalyse)
Die Cockpit-Architektur ist **bereits konsolidiert**:
- 5 Areas + Home (Context, Memory, Protection, Proof, Project Map)
- Jede Area hat Tabs via `cockpit-area-tabs.js`
- Lazy Loading über `makeViewLoader` + `registerLoader`
- Simple Mode (Home only) + Pro Mode (alle Areas)

Die ursprüngliche Empfehlung "22 Views → 7 Views" war **falsch** — die Tab-Konsolidierung existierte bereits. Das echte Problem war: **Kernel-APIs waren nicht integriert**.

### C1: Kernel API Integration ✅ (2026-07-23)
**Deliverables:**
- `rust/src/dashboard/routes/kernel.rs` (79 LOC) — konsolidierte Kernel-API
- `cockpit-health.js`: "Kernel" Tab (Health Hero, Savings, Provider Distribution, Subsystems)
- `cockpit-overview.js`: Kernel-Chip im StatusStrip (healthy/degraded/off)
- `cockpit-roi.js`: Provider Distribution Tabelle
- `/api/kernel` live getestet: JSON mit 6 Subsystemen, Provider Stats, Evidence
- Pushed: GitHub ✓ + GitLab ✓

### C2: Clippy Zero-Warning Policy ✅ (2026-07-23)
- 27 vorbestehende Clippy-Warnings in 21 Dateien behoben
- `cargo clippy --all-targets -- -D warnings` passiert sauber
- 9051 Tests ✓ (1 flaky pre-existing: prefix_replay)

### Verbleibende Optimierungen (optional)
- CSS Audit (1616 Zeilen, ~30% vermutlich ungenutzt)
- Nav-Metadata SSOT (Labels dreifach definiert)
- Component-Level Refactoring (Graph 1891L, Live 1227L)

