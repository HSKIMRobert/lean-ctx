
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

## Premium Production Readiness (E-Phasen)

Stand: 2026-07-28

### D6-D9: LOC Splits (4 Runden, 16 Agents) ✅
- Alle Rust-Dateien unter 1500 LOC (Gate-Compliance)
- Nur `shell_allowlist/tests.rs` (1427 LOC) noch über 1400 LOC
- Flaky Test Fix: `prefix_replay::tests::append_only_detection_works`

### E1: Test-File LOC Splits (4 Agents) ✅
- `compress/tests.rs`, `hook_handlers/tests.rs`, `config/tests.rs`, `ctx_read/tests.rs` gesplittet
- 9074 Tests, 0 Failures

### E2: ETPAO Runtime Baseline (4 Agents) ✅ (2026-07-27)
- `savings_ledger/etpao.rs`: RuntimeEtpao aus echten Events (Baseline vs. Delivered, per-Tool)
- `telemetry.rs`: ObservedEfficiency Export (Cache Hit Rate, Request Count)
- `ctx_gain.rs`: ETPAO-Section mit Live-Daten
- `efficiency_analyzer.rs`: 5 E2E-Testszenarien
- Flaky Test Fix: `mutate_locked_preserves_successive_agent_episodes`
- 9095 Tests, 0 Clippy Warnings

### E3: Multi-Layer Cache Pipeline (6 Agents) ✅ (2026-07-27)
- **Root Cause**: `telemetry.record_cache()` wurde NIE aus Produktion aufgerufen → ~0.7% Hit Rate
- SessionCache + ContentCache hits → zentrale Telemetrie verdrahtet
- ResponseCache aktiviert für deterministische Tool-Calls
- Cache Warming Modul (`cache/warming.rs`)
- Multi-Layer Cache Dashboard (Session/Content/Response/Overall)
- E2E Pipeline-Tests (`cache/pipeline_tests.rs`)
- 879 neue LOC, 9137 Tests, 0 Clippy Warnings

### E4: A2A Transport Hardening (5 Agents) ✅ (2026-07-28)
- `a2a/remote_transport.rs` (342 LOC): HTTP Transport mit Retry + Timeout + Auth
- `a2a/health.rs` (145 LOC): Transport Health Probes (Ready/Degraded/Unavailable)
- `a2a/relay.rs` (149 LOC): Multi-Hop Relay Chain + Cycle-Detection
- `a2a/budget_cascade.rs` (201 LOC): Token Budget Parent→Child Cascade
- `a2a/telemetry.rs` (139 LOC): Transport Delivery Metrics
- 976 neue LOC, 9147 Tests, 0 Clippy Warnings

### E5: Doku-SSOT-Update ✅ (2026-07-28)
- Memory-Bank (activeContext, progress) auf E2-E4-Stand
- OCLA-UMBAU-ZIEL.md mit E-Phasen aktualisiert
- Requirements Matrix IN-05 A2A-Status aktualisiert

## Gesamtstatistik E-Phasen

| Phase | Agents | Neue LOC | Tests Total |
|---|---|---|---|
| E1 | 4 | ~600 | 9074 |
| E2 | 4 | ~530 | 9095 |
| E3 | 6 | ~879 | 9137 |
| E4 | 5 | ~976 | 9147 |
| **Gesamt** | **19** | **~2985** | **9147** |
