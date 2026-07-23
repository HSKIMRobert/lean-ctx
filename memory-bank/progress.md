
### R32 — Provider Live-Wiring + Envelope Bridge + Azure (5 Agents)
- UsageParity: provider_kind_from_label() + real_usage_to_envelope() + label_from_base_url()
- EnvelopeBridge: record_proxy_envelope() + record_mcp_envelope() + provider_stats() Pipeline
- ProviderMetricsE2E: 8 E2E-Tests für detect→envelope→record→stats Pipeline
- UsageAzure: is_azure_response() + normalize_azure_model() + parse_azure_usage()
- ProviderDisplay: format_provider_table() + provider_summary_oneliner() + provider_json()
- Wiring: Provider::from_label +Azure, DashboardReport +provider_distribution
- 478 Kernel-Tests + 17 Proxy-Tests, 0 Clippy Warnings
- **MEILENSTEIN: Provider-Pipeline vollständig verdrahtet — detect → envelope → stats → dashboard**
