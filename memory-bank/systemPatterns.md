# System Patterns

## SSOT: Tool-Manifest
- **Source**: `rust/src/tool_defs.rs` (`granular_tool_defs` → 42, `unified_tool_defs` → 5)
- **Generator**: `rust/src/bin/gen_mcp_manifest.rs`
- **Output (repo-tracked)**: `website/generated/mcp-tools.json`
- **CI Gate**: `rust/tests/mcp_manifest_up_to_date.rs`

## Tool Dispatch
- MCP stdio: `LeanCtxServer` implementiert `rmcp::handler::server::ServerHandler` (in `rust/src/server.rs`).
- Unified Tool `ctx` dispatcht intern auf `ctx_*`.

## Rails / Harness Layer
- `core/workflow/*`: deterministische State Machine mit erlaubten Tools + Evidence Requirements.
- Gatekeeper: filtert `list_tools` + blockt `call_tool` (immer erlaubt: `ctx`, `ctx_workflow`).
- Evidence Store: Tool-Receipts (Input/Output Hash) + Transition-Gates.
- Persistence: `~/.lean-ctx/workflows/active.json` (atomares Schreiben via tmp+rename).

## Observability (local-first)
- `ctx_cost`: CostStore persistiert, wird zentral in `server.rs` befüllt.
- `ctx_heatmap`: Heatmap wird bei Reads instrumentiert (u.a. `ctx_read`, `ctx_multi_read`).

## Transport Layer
- **stdio**: Standard MCP Transport (`rmcp::transport::io`)
- **Streamable HTTP**: `lean-ctx serve` via `lean_ctx::http_server` + `rmcp::transport::StreamableHttpService`
- **REST Endpoints**: `/v1/manifest`, `/v1/tools` (paginated), `/v1/tools/call` (mit Timeout)
- **Defaults**: loopback bind, Host-header validation, optional Bearer Auth bei non-loopback.

## Library API
- `lean_ctx::engine::ContextEngine`: einbettbare API für Harnesses/Orchestratoren.
- `call_tool_text()`, `call_tool_value()`, `call_tool_result()`, `manifest()`.

## Session Cache
- Files im Speicher gecached mit MD5 Hash
- Re-reads kosten 13 Tokens statt Tausende
- Auto-clear nach 5 min Inaktivität (konfigurierbar via `LEAN_CTX_CACHE_TTL`)
- Auto-checkpoint alle 10 Tool-Calls (konfigurierbar via `LEAN_CTX_CHECKPOINT_INTERVAL`)

## Pattern Compression (Shell Hook)
- `patterns/mod.rs` routet Commands zu spezifischen Pattern-Modulen
- 47 Module mit 90+ Patterns über 34 Kategorien
- Fallback: spezifisches Pattern → JSON Schema → Log Dedup → generische Truncation

## Version Management
- Version ist **hardcoded** in 7+ Stellen (kein `env!("CARGO_PKG_VERSION")`)
- Siehe `memory-bank/release-checklist.md` für alle Stellen

## USD Berechnung
- Standard Rate: **$2.50 pro 1M Tokens**
- Formel: `saved_tokens * 2.50 / 1_000_000`
