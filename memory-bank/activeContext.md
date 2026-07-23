# Active Context

Stand: 2026-07-23T12:20+02:00

## Aktueller Fokus

R32 abgeschlossen — Provider Live-Wiring + Envelope Bridge + Azure Support.

## Letzte Änderungen (R32)

- **usage_parity.rs** (113 LOC): provider_kind_from_label + real_usage_to_envelope
- **envelope_bridge.rs** (186 LOC): record_proxy/mcp_envelope + provider_stats
- **provider_metrics_e2e.rs** (161 LOC): 8 E2E Pipeline-Tests
- **usage_azure.rs** (128 LOC): Azure-Response-Erkennung + Usage-Parsing
- **provider_display.rs** (165 LOC): Formatierte Provider-Tabelle + JSON
- **dashboard_report.rs**: +provider_distribution Feld
- **usage.rs**: Provider::from_label +Azure Mapping
- **478 Kernel-Tests + 17 Proxy-Tests**, 0 Clippy

## Architektur-Status

### OCLA: P0-P9, P11 = 100%
### Context Kernel: 32 Runden, 478+ Tests
### Provider Pipeline: detect → envelope → bridge → stats → dashboard ✓
### Unterstützte Provider: OpenAI, Anthropic, Gemini, Bedrock, Azure, OpenRouter, Local

## Nächste Schritte

1. **R33**: envelope_from_usage in proxy response-path verdrahten (RealUsage → TokenEnvelope → record)
2. **R34**: Dashboard HTML/HTMX UI für /v1/kernel/dashboard
3. **R35**: Cost-Tracking mit echten Provider-Rechnungen (pricing tables)
