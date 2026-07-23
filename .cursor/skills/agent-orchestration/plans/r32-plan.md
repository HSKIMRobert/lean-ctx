# R32 — Provider Detection Live-Wiring + Envelope Bridge + Usage Parity

## Ziel
`detect_provider()` und `envelope_from_usage()` in die Live-Pfade verdrahten:
1. Proxy forward → ProviderKind auf jedem Request
2. Usage-Scanner → Azure Support + ProviderKind-Integration
3. Envelope-Bridge: RealUsage → TokenEnvelope Konvertierung
4. Provider Stats: Per-Provider Metriken im Dashboard

## Kontext (OSS)
- `provider_parity.rs` hat `detect_provider(base_url)` und `envelope_from_usage(kind, model, json)`
- `usage.rs` hat `Provider` enum (Anthropic/OpenAi/Gemini) — braucht Azure + Bedrock Mapping
- `proxy_bridge.rs` empfängt `ProxyRequestData` mit `provider: Option<String>`
- `forward/mod.rs` hat `provider_label: &str` auf jedem Request
- Alle neuen Files ≤ 200 LOC, nur OSS-Code

## Agent-Aufträge

### Agent 01 — Usage Provider Parity (`proxy/usage_parity.rs`, max 150 LOC)
Erweitert den Usage-Scanner für Azure und Bedrock, bridged zu ProviderKind.

**Functions:**
- `pub fn provider_kind_from_label(label: &str) -> ProviderKind`
  Maps proxy `provider_label` strings to kernel `ProviderKind`:
  - "Anthropic" → Anthropic
  - "OpenAI" | "ChatGPT" → OpenAi
  - "Gemini" → Gemini
  - "Bedrock" → Bedrock
  - "Azure" → Azure
  - starts with "openrouter" (case-insensitive) → OpenRouter
  - else → Unknown

- `pub fn real_usage_to_envelope(usage: &super::usage::RealUsage, label: &str) -> TokenEnvelope`
  Converts proxy's RealUsage to kernel's TokenEnvelope:
  - Maps all token fields 1:1 (u64→usize)
  - Sets provider from provider_kind_from_label
  - Sets model from usage.model
  - Sets estimated_cost_usd from usage.provider_cost_usd

- `pub fn label_from_base_url(base_url: &str) -> &'static str`
  Maps base_url to provider_label string for use in forward path:
  - Uses detect_provider() internally
  - Returns "Anthropic", "OpenAI", "Gemini", "Bedrock", "Azure", "OpenRouter", "Local", "Unknown"

**Tests (≥6):**
1. `kind_from_anthropic_label` → Anthropic
2. `kind_from_openai_label` → OpenAi
3. `kind_from_bedrock_label` → Bedrock
4. `kind_from_azure_label` → Azure
5. `real_usage_converts` — full RealUsage → TokenEnvelope with all fields
6. `label_from_url_openai` → "OpenAI"
7. `label_from_url_bedrock` → "Bedrock"

### Agent 02 — Envelope Bridge (`context_kernel/envelope_bridge.rs`, max 150 LOC)
Bridge between proxy usage data and kernel TokenEnvelope pipeline.

**Functions:**
- `pub fn record_proxy_envelope(envelope: &TokenEnvelope)`
  Records a TokenEnvelope from the proxy path into kernel evidence:
  - Calls envelope_wiring::record_from_proxy_dispatch(...)
  - Updates usage_normalizer with the envelope
  - Thread-safe via existing kernel locks

- `pub fn record_mcp_envelope(tool_name: &str, envelope: &TokenEnvelope)`
  Records a TokenEnvelope from MCP tool calls:
  - Calls envelope_wiring::record_from_tool_dispatch(...)
  - Updates usage_normalizer

- `pub fn provider_stats() -> Vec<ProviderStat>`
  Aggregates per-provider token statistics:
  ```rust
  #[derive(Debug, Clone, Serialize)]
  pub struct ProviderStat {
      pub provider: ProviderKind,
      pub request_count: usize,
      pub total_input: usize,
      pub total_output: usize,
      pub total_cache_read: usize,
      pub avg_input: usize,
  }
  ```

- `pub fn reset()`

**Tests (≥5):**
1. `record_proxy_updates_stats` — record 3 envelopes, stats show correct counts
2. `record_mcp_updates_stats` — record MCP envelopes, stats correct
3. `provider_stats_multi` — different providers, each tracked separately
4. `reset_clears_stats` — after reset, all zeros
5. `empty_stats_safe` — provider_stats() returns empty vec, no panic

### Agent 03 — Provider Metrics E2E (`context_kernel/provider_metrics_e2e.rs`, max 200 LOC, #[cfg(test)] only)
End-to-end tests proving provider detection → envelope → stats pipeline.

**Tests (≥7):**
1. `openai_request_flow` — detect → envelope → record → stats shows OpenAi
2. `anthropic_request_flow` — detect → envelope → record → stats shows Anthropic
3. `bedrock_request_flow` — detect → envelope → record → stats shows Bedrock
4. `azure_request_flow` — detect → envelope → record → stats shows Azure
5. `multi_provider_stats` — mix of providers → each tracked correctly
6. `envelope_bridge_integrates_with_evidence` — record_proxy_envelope triggers evidence_wiring
7. `dashboard_shows_provider_stats` — dashboard_report includes provider distribution
8. `reset_isolates_tests` — full reset clears all provider state

### Agent 04 — Usage Azure Extension (`proxy/usage_azure.rs`, max 150 LOC)
Extends usage parsing for Azure-specific response shapes.

**Functions:**
- `pub fn is_azure_response(headers: &HeaderMap) -> bool`
  Checks for Azure-specific headers:
  - `x-ms-region` present → true
  - `x-ratelimit-remaining-requests` with azure pattern → true

- `pub fn extract_azure_model(model: &str) -> String`
  Normalizes Azure model names:
  - "gpt-4o-2024-05-13" stays as-is
  - Strips deployment name prefix if present
  - Returns cleaned model ID

- `pub fn azure_usage_from_json(usage: &Value) -> Option<RealUsage>`
  Parses Azure's OpenAI-compatible usage with Azure-specific extensions:
  - Standard fields: prompt_tokens, completion_tokens
  - Azure extensions: prompt_tokens_details.cached_tokens
  - Content filter tokens (if present, subtract from billable)

**Tests (≥5):**
1. `is_azure_by_region_header` → true
2. `is_not_azure_without_headers` → false
3. `extract_model_passthrough` — standard name unchanged
4. `azure_usage_standard` — prompt_tokens + completion_tokens parsed
5. `azure_usage_with_cache` — cached_tokens extracted

### Agent 05 — Provider Display for Dashboard (`context_kernel/provider_display.rs`, max 150 LOC)
Formatted provider statistics for dashboard and CLI display.

**Functions:**
- `pub fn format_provider_table(stats: &[super::envelope_bridge::ProviderStat]) -> String`
  ```
  Provider     │ Reqs │    Input │   Output │   Cache │ Avg In
  ─────────────┼──────┼──────────┼──────────┼─────────┼───────
  OpenAI       │   42 │  125,400 │   31,200 │  45,000 │  2,986
  Anthropic    │   18 │   54,000 │   12,600 │  22,000 │  3,000
  ─────────────┼──────┼──────────┼──────────┼─────────┼───────
  Total        │   60 │  179,400 │   43,800 │  67,000 │  2,990
  ```

- `pub fn provider_summary_oneliner(stats: &[super::envelope_bridge::ProviderStat]) -> String`
  → "3 providers: OpenAI(42) Anthropic(18) Gemini(5)"

- `pub fn provider_json(stats: &[super::envelope_bridge::ProviderStat]) -> Value`
  JSON array of provider stats for API responses.

**Tests (≥4):**
1. `format_table_with_data` — contains "Provider", "│", numbers
2. `format_table_empty` — returns "No provider data" or similar
3. `oneliner_format` — contains provider names and counts
4. `json_valid` — parses as valid JSON array

## Manuelles Wiring (nach Agent-Merge)

### 1. Usage Provider::from_label erweitern
In `usage.rs`: Add `"Azure"` mapping in `Provider::from_label` → `Self::OpenAi` (same wire shape).

### 2. Forward path: inject ProviderKind
In `forward/xlat.rs` `process_response`: Convert provider_label → ProviderKind via usage_parity.

### 3. Dashboard enrichment
Wire `envelope_bridge::provider_stats()` into `dashboard_report::generate_report()`.

## Quality Gate
- `cargo fmt --check` + `cargo clippy --all-features -- -D warnings`
- `cargo test --all-features --lib -- context_kernel` (target: 490+ tests)
- LOC-Gate: alle neuen Files ≤ 200 LOC
- OSS-only: kein kommerzieller Code, keine memory-bank push auf GitHub
