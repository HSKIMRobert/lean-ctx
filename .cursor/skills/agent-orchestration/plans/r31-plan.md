# R31 — Provider Parity + Production Evidence + Dashboard

## Ziel
Die drei grössten verbleibenden Lücken schliessen:
1. **Provider Envelope Parity**: ProviderKind um Bedrock/Azure erweitern, Envelope in allen Provider-Pfaden
2. **Production Evidence**: Air-Gap Test, Performance Benchmark, Golden Trace Corpus
3. **Kernel Dashboard**: Health-Summary als strukturierter Report statt nur JSON

## Kontext
- Bedrock-Adapter (1009 LOC) + Google-Adapter (516 LOC) existieren bereits
- Azure läuft über `WireShape::OpenAi` + `[[proxy.providers]]` Config
- `ProviderKind` hat: OpenAi, Anthropic, Gemini, OpenRouter, Local, Unknown — fehlen: **Bedrock, Azure**
- `TokenEnvelope` existiert mit allen Feldern, aber nicht für alle Provider verdrahtet
- `/v1/kernel/health` gibt JSON, aber kein strukturiertes Dashboard
- 419 Kernel-Tests bestehen

## Agent-Aufträge

### Agent 01 — Provider Envelope Parity (`context_kernel/provider_parity.rs`, max 150 LOC)
Erweitert ProviderKind + TokenEnvelope für alle unterstützten Provider.

**Changes:**
1. In `token_envelope.rs`: Add `Bedrock` and `Azure` variants to `ProviderKind`
2. New file `provider_parity.rs`:

**Functions:**
- `pub fn detect_provider(base_url: &str, model: &str) -> ProviderKind`
  → Detects provider from URL patterns:
  - `api.openai.com` → OpenAi
  - `api.anthropic.com` → Anthropic
  - `generativelanguage.googleapis.com` → Gemini
  - `bedrock-runtime.*.amazonaws.com` → Bedrock
  - `*.openai.azure.com` / `*.services.ai.azure.com` → Azure
  - `openrouter.ai` → OpenRouter
  - `localhost` / `127.0.0.1` → Local
  - else → Unknown

- `pub fn envelope_from_usage(provider: ProviderKind, model: &str, usage: &serde_json::Value) -> TokenEnvelope`
  → Parses provider-specific usage JSON into canonical TokenEnvelope:
  - OpenAI: `usage.prompt_tokens`, `usage.completion_tokens`, `usage.prompt_tokens_details.cached_tokens`
  - Anthropic: `usage.input_tokens`, `usage.output_tokens`, `usage.cache_read_input_tokens`
  - Gemini: `usageMetadata.promptTokenCount`, `usageMetadata.candidatesTokenCount`
  - Bedrock: `usage.inputTokens`, `usage.outputTokens`
  - Azure: same as OpenAI (same wire shape)

- `pub fn provider_display_name(kind: ProviderKind) -> &'static str`
- `pub fn all_supported_providers() -> &'static [ProviderKind]`

**Tests (≥5):**
1. `detect_openai` — api.openai.com → OpenAi
2. `detect_anthropic` — api.anthropic.com → Anthropic
3. `detect_bedrock` — bedrock-runtime.us-east-1.amazonaws.com → Bedrock
4. `detect_azure` — my-resource.openai.azure.com → Azure
5. `envelope_from_openai_usage` — parses prompt_tokens/completion_tokens/cached
6. `envelope_from_anthropic_usage` — parses input_tokens/output_tokens/cache_read
7. `detect_unknown` — random URL → Unknown

### Agent 02 — Air-Gap E2E (`context_kernel/airgap_e2e.rs`, max 200 LOC, #[cfg(test)] only)
Proves kernel and local features work without network.

**Tests:**
1. `kernel_works_without_network` — Initialize kernel, run dedup, search, evidence, health — all work
2. `config_loads_without_cloud` — ConfigBridge loads from local TOML, no network calls
3. `ledger_records_offline` — SavingsLedger records events purely locally
4. `bounce_tracker_works_locally` — Full bounce detection cycle without any remote
5. `envelope_works_without_provider` — TokenEnvelope with ProviderKind::Unknown still processes
6. `health_api_works_offline` — /v1/kernel/health returns valid JSON without network
7. `all_kernel_modules_default_safe` — Every reset() + summary/report function returns valid defaults

### Agent 03 — Performance Benchmark (`context_kernel/perf_benchmark.rs`, max 200 LOC, #[cfg(test)] only)
Quantitative performance tests for kernel hot paths.

**Tests:**
1. `dedup_latency_under_1ms` — try_dedup for 100 files completes in <100ms total
2. `schema_opt_latency_under_5ms` — optimize_descriptions for 50 tools in <250ms
3. `evidence_recording_throughput` — 10k record_tool_call in <100ms
4. `health_report_latency` — kernel_health() in <1ms
5. `envelope_creation_throughput` — 10k TokenEnvelope creations in <50ms
6. `concurrent_evidence_safe` — 4 threads recording simultaneously, no panics
7. `search_dedup_scales` — 1000 unique queries + 100 repeats, detection correct

### Agent 04 — Dashboard Report (`context_kernel/dashboard_report.rs`, max 200 LOC)
Structured, human-readable kernel dashboard report.

**Types:**
```rust
#[derive(Debug, Clone, Serialize)]
pub struct DashboardReport {
    pub version: &'static str,
    pub uptime_secs: u64,
    pub kernel_enabled: bool,
    pub health_status: &'static str, // "healthy" | "degraded" | "disabled"
    pub subsystems: Vec<SubsystemStatus>,
    pub token_savings: TokenSavingsSummary,
    pub provider_distribution: Vec<ProviderUsage>,
    pub recent_activity: RecentActivity,
}
```

**Functions:**
- `pub fn generate_report() -> DashboardReport` — aggregates from all kernel modules
- `pub fn format_report(report: &DashboardReport) -> String` — human-readable multi-line
- `pub fn report_json(report: &DashboardReport) -> String` — JSON serialization

**Tests (≥3):**
1. `report_has_all_sections` — version, health, subsystems all populated
2. `format_report_readable` — contains "Kernel", section headers, numbers
3. `report_json_valid` — parses as valid JSON with expected fields

### Agent 05 — Golden Provider Traces (`context_kernel/provider_traces.rs`, max 200 LOC, #[cfg(test)] only)
Deterministic golden test corpus for provider-specific usage parsing.

**Tests:**
1. `openai_chat_completion_usage` — Standard OpenAI response → correct envelope
2. `openai_cached_usage` — OpenAI with cached_tokens → cache_read_tokens populated
3. `anthropic_messages_usage` — Anthropic response → correct envelope
4. `anthropic_cache_usage` — Anthropic with cache_read/write → correct envelope
5. `gemini_usage_metadata` — Gemini usageMetadata → correct envelope
6. `bedrock_invoke_model_usage` — Bedrock response → correct envelope
7. `azure_openai_usage` — Azure (OpenAI-shaped) → correct envelope
8. `empty_usage_safe` — Missing/null usage fields → zero-valued envelope, no panic
9. `unknown_provider_passthrough` — Unknown provider → envelope with Unknown kind

## Manuelles Wiring (nach Agent-Merge)

### 1. ProviderKind erweitern (`token_envelope.rs`)
Add `Bedrock` and `Azure` variants:
```rust
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Gemini,
    Bedrock,    // NEW
    Azure,      // NEW
    OpenRouter,
    Local,
    #[default]
    Unknown,
}
```

### 2. Provider detection in forward.rs
Wire `provider_parity::detect_provider()` into the proxy forward path
so every request gets a correct ProviderKind on the envelope.

### 3. Dashboard endpoint enhancement
Wire `dashboard_report::generate_report()` into `kernel_api::dashboard()` handler.

## Quality Gate
- `cargo fmt --check`
- `cargo clippy --all-features -- -D warnings`
- `cargo test --all-features --lib -- context_kernel` (target: 450+ tests)
- `codesign --force --sign -` after build (macOS Sequoia)
- LOC-Gate: alle neuen Files ≤ 200 LOC
- dev-install + smoke test
