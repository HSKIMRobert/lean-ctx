# Tech Context

## Stack

- **Rust 2021 Edition**: Core runtime, MCP server, CLI, all tools (`rust/`)
- **Tokio**: Async runtime (multi-threaded, configurable workers)
- **Axum**: HTTP server (proxy, team-server)
- **rmcp**: MCP protocol handler (stdio + Streamable HTTP)
- **tree-sitter**: AST parsing for 18 languages (feature-gated)
- **Astro**: Website/Docs (branch `deploy`)
- **Node/TypeScript**: SDK + Cookbook (`cookbook/`)

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `serde` / `serde_json` / `toml` | Serialization |
| `tokio` | Async runtime |
| `rmcp` | MCP transport |
| `axum` + `tower-http` | HTTP/Proxy |
| `rusqlite` | Property Graph, optional persistence |
| `tree-sitter-*` (18 langs) | Signatures, imports, AST |
| `zstd` | Index compression |
| `subtle` | Constant-time auth comparison |
| `getrandom` | CSPRNG token generation |
| `tiktoken-rs` | Token counting |
| `chrono` | Timestamps |
| `ignore` | .gitignore-aware directory walking |

## Build & Quality Gates

```bash
lean-ctx stop                              # Stop running processes
cd rust && cargo fmt                        # Format
cd rust && cargo clippy -- -W clippy::all   # Lint (pedantic)
cd rust && cargo test --lib                 # Unit tests (4114+)
cd rust && cargo test --tests              # Integration tests (91 files)
cd rust && cargo build --release            # Release build (~2.5min)
lean-ctx dev-install                        # Atomic: stop→build→install→restart
```

## Environment Variables (wichtig)

| Var | Effect |
|-----|--------|
| `LEAN_CTX_DISABLED=1` | Disables lean-ctx hooks during build |
| `LEAN_CTX_PROXY_PORT` | Override proxy port |
| `LEAN_CTX_SHELL_ALLOWLIST` | Comma-separated allowed commands |
| `LEAN_CTX_NO_JAIL` | Disable PathJail (development only) |
| `LEAN_CTX_META=1` | Enable meta-messages in tool output |
| `LEAN_CTX_WORKER_THREADS` | Server thread count |
| `GITHUB_TOKEN` | Aktiviert GitHub Provider (Issues, PRs, Actions) |
| `GITLAB_TOKEN` | Aktiviert GitLab Provider (Issues, MRs, Pipelines) |
| `JIRA_TOKEN` + `JIRA_BASE_URL` | Aktiviert Jira Provider |
| `DATABASE_URL` | Aktiviert PostgreSQL Provider |

## SSOT Artefakte

- Tool Registry: `rust/src/server/registry.rs` (runtime SSOT, 62 Tools)
- Tool Schemas: `rust/src/tool_defs/granular.rs`
- Website Manifest: `website/generated/mcp-tools.json`
- Contracts: `rust/src/core/contracts.rs` + `CONTRACTS.md`
- Provider Config: `[providers]` in `~/.lean-ctx/config.toml` + `[providers.mcp_bridges]`

## Token-Control-Platform Zielarchitektur

- OCLA Rust Traits plus versionierter REST/gRPC Wire Contract
- Canonical Token Envelope für Request, Stream, Tool Call, Usage und Error
- OpenAPI/Protobuf + SDKs: Java, .NET, Python, TypeScript, Go, Rust
- Customer-owned Gateway/Sidecar/in-process Deployment
- Provider: OpenAI-kompatibel, Anthropic, Azure, Bedrock, Vertex/Gemini, lokal
- Streaming + Tool Calls müssen semantisch erhalten bleiben
- Capability Discovery, Contract-/Schema-Version, Health und Limits
- Typed Error Model, Correlation, Idempotency, Deadlines und Cancellation
- HA, horizontale Skalierung, Backpressure, Circuit Breaker, Rate Limits
- policy-spezifisches Fail-open/Fail-closed und Shadow Mode
- Unified Ledger mit MeasurementMethod, Quality, Attribution und Approval

Der Data Plane darf keine Runtime-Abhängigkeit auf Thinkery Cloud, AI Value Gate
oder Lizenzserver besitzen.

## Transformation Gates

- W0/G0: codeverankerte Reality Baseline
- W1/G1: OCLA + Envelope Contract
- W2/G2: Evidence Foundation
- W3/G3: Compression Fidelity/Quality
- W4/G4: Universal Data Plane
- W5/G5: Identity/Policy Control
- W6/G6: Wire/SDK/External Certification
- W7/G7: vier Optimization Dimensions
- W8/G8: Security/SRE/Operations PRR
- W9/G9: Commercial/Settlement Traceability
- W10/G10: Pilot + second fork-free deployment + GA

## IDE Integration Points

| IDE | Config File | Transport |
|-----|-------------|-----------|
| Cursor | `.vscode/mcp.json` + `.cursor/rules/` | stdio |
| Claude Code | `AGENTS.md` + `.claude/settings.local.json` | stdio |
| VS Code (Copilot) | `.vscode/mcp.json` | stdio |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` | stdio |
| Zed | `~/.config/zed/settings.json` | stdio |

## Release Flow

1. Version bump: `Cargo.toml` + `package.json` + AUR
2. `cargo test --lib` + `cargo clippy`
3. Git tag `v3.x.x` + push GitHub
4. GitHub Actions builds binaries (Linux/macOS/Windows)
5. AUR: update PKGBUILD + .SRCINFO
6. Website: update version.txt on `deploy` branch
