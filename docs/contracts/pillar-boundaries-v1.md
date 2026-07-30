# Pillar Boundaries Contract v1

> Architectural contract defining the three lean-ctx pillars and their
> dependency rules. CI-enforced via `contracts_frozen.rs`.

## Pillars

### Engine (always compiled)

The developer-facing context compression layer. All features work locally,
offline, with zero telemetry.

**Top-level modules:** `core`, `tools`, `server`, `engine`, `tool_defs`,
`instructions`, `mcp_stdio`, `hooks`, `hook_handlers`, `rules_inject`,
`rewrite_registry`, `shell`, `shell_hook`, `dashboard`, `tui`, `terminal_ui`,
`lsp`, `compound_lexer`, `marked_block`, `dropin`, `heatmap`, `token_report`,
`daemon`, `daemon_autostart`, `daemon_client`.

**Feature flags:** none (always compiled).

### Gateway (lean-ctx-enterprise)

The org-wide LLM reverse proxy with usage tracking, budget enforcement, and
FinOps dashboards.

**Top-level modules:** `proxy`, `proxy_autostart`, `proxy_setup` (OSS);
`gateway_server` (enterprise).

**Feature flags:** `engine-integration` (enterprise) — links OSS proxy to
enterprise gateway admin + usage store.

### Cloud (lean-ctx-enterprise)

Hosted coordination: accounts, team provisioning, knowledge sync, billing
edge, context package registry.

**Top-level modules:** `cloud_server`, `cloud_client`, `cloud_sync`,
`http_server` (team/billing surfaces) — all in `lean-ctx-enterprise`.

**Feature flags:** enterprise build only (ADR-023).

### Shared (always compiled)

CLI, IPC, config, diagnostics — consumed by all three pillars.

**Top-level modules:** `cli`, `config_io`, `ipc`, `doctor`, `setup`,
`status`, `report`, `uninstall`.

## Dependency rules

1. **Engine depends on nothing** — it is the foundation.
2. **Gateway depends on Engine** — the proxy compresses prompts using
   `core::compressor`.
3. **Cloud depends on Engine** — sync and billing reference `core::config`,
   `core::savings_ledger`.
4. **Gateway ↔ Cloud are independent** — no direct imports between
   OSS `proxy` and enterprise `gateway_server` / `cloud_server` / `cloud_sync`.
5. **`http_server` (OSS)** — Engine HTTP MCP transport only. Team/Cloud
   HTTP surfaces live in `lean-ctx-enterprise`.

## Cross-pillar coupling (documented exceptions)

### proxy ↔ gateway_server (lean-ctx-enterprise)

The self-hosted org gateway runs as a single enterprise process:
- `gateway_server::serve` (enterprise) calls `proxy::start_proxy` (OSS)
- `proxy` mounts enterprise `gateway_server::user_api` and
  `gateway_server::mcp::proxy` routes via `engine-integration`

This cross-repo dependency is intentional (ADR-023) and documented in both
repositories.

## Local-Free Invariant

Every feature in every pillar works self-hosted for free. Commercial tiers
(Cloud) add hosting and support, never capabilities. CI enforces this via
the `local_free_invariant` test.

## Naming convention

| Old name | New name | Reason |
|----------|----------|--------|
| `core::gateway` | `core::mcp_catalog` | Avoid collision with `gateway_server` (the LLM Gateway) |
