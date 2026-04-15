# Tech Context

## Runtime / Language
- **Rust 2021**, Single-binary CLI + Library (`lean_ctx`).
- **MCP** via `rmcp` (stdio + Streamable HTTP transport).

## Wichtige Dependencies
- `rmcp` (server + stdio transport + streamable-http-server transport)
- `tokio` (async runtime, incl. signal für graceful shutdown)
- `axum` + `tower-http` (HTTP server, optional via `http-server` Feature)
- `rusqlite` mit `bundled` (cross-platform SQLite)
- `tree-sitter-*` (optional, aktiviert via default features)
- `tiktoken-rs` (Token counting, o200k_base encoding)
- `serde` + `serde_json` + `toml` (Serialization)
- `md-5` (Hashing für Cache + Evidence Receipts)
- `chrono` (Timestamps)
- `anyhow` (Error handling)

## Feature Flags
- `default = ["tree-sitter", "embeddings", "http-server"]`
- `http-server = ["dep:axum", "dep:tower-http"]`
- `cloud-server` (extends `http-server` + deadpool-postgres, lettre, etc.)

## Website
- Astro SSG (separater Deploy-Worktree/Branch `deploy`)
- Tool-Counts/Read-Modes werden aus `website/generated/mcp-tools.json` gerendert
- Node.js >= 22.12.0 erforderlich
- Tailwind CSS 4.x

## Build/Tests
- `cargo test` muss grün sein (Unit + Integration, aktuell 769+ lib + ~85 integration)
- Manifest drift wird durch `mcp_manifest_is_up_to_date` verhindert
- `cargo run --bin gen_mcp_manifest` regeneriert SSOT

## Release Profile
```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

## CI/CD
- **GitHub Actions**: `.github/workflows/release.yml` — getriggert bei `v*` Tags, 5 Targets
- **GitLab CI**: `.gitlab-ci.yml` — cargo check + website deploy

## Git Remotes
| Name | URL | Verwendung |
|------|-----|------------|
| `origin` | `https://gitlab.pounce.ch/root/lean-ctx.git` | GitLab (primary für CI) |
| `github` | `git@github.com:yvgude/lean-ctx.git` | GitHub (public, releases) |

## Package Registries
| Registry | URL |
|----------|-----|
| crates.io | https://crates.io/crates/lean-ctx |
| Homebrew | https://github.com/yvgude/homebrew-lean-ctx |
| AUR lean-ctx | https://aur.archlinux.org/packages/lean-ctx |
| AUR lean-ctx-bin | https://aur.archlinux.org/packages/lean-ctx-bin |
| GitHub Releases | https://github.com/yvgude/lean-ctx/releases |
