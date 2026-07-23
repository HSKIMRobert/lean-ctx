
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

## Cockpit Architecture Audit

Stand: 2026-07-23

### Bestandsaufnahme
- 23 JS Components (518 KB), 89 KB CSS, 49 KB HTML
- 22 Views in der Navigation
- 6 Lib-Dateien (61 KB), 4 Tests
- Backend: dashboard/mod.rs (1026 LOC)

### Kritische Findings (F1-F10)
- F1: Overview dupliziert 6 APIs die andere Views auch konsumieren
- F2: Context + Commander + Compression = 3 Views für dasselbe Thema (94KB)
- F3: Knowledge + Memory + Search = 3 Views für "persistent Memory" (62KB)
- F4: ROI + Remaining + Leaderboard = 3 Views für "Savings" (85KB)
- F5: Health + Protection = 2 Views für "System Health" (29KB)
- F6: Graph (79KB!!!) + Architecture + Explorer (defekt) = Code-Struktur
- F7: 0 Consumer von /v1/kernel/* APIs (R31-R33 unsichtbar)
- F8: Nav-Metadata dreifach dupliziert
- F9: 89KB CSS ohne Methodik
- F10: shared.js (26KB) Grab-Bag

### Premium-Ziel: 22 Views → 7 Views
Home | Context (3 Tabs) | Knowledge (2 Tabs) | Savings (3 Tabs) | System (3 Tabs) | Code (2 Tabs) | Activity (3 Tabs)

### Consolidation-Phasen
C1: Context (HOCH) | C2: Savings (HOCH) | C3: Knowledge (MITTEL) | C4: System+Kernel (MITTEL) | C5: Code (NIEDRIG) | C6: Activity (NIEDRIG) | C7: Infrastructure (MITTEL) | C8: Home (NIEDRIG)
