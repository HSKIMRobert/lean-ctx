# Journey 21 — lean-md (Addon Integration)

> lean-md is an **external lean-ctx addon** — a macro/directive Markdown renderer.
> It lives in its own repository (`dasTholo/lean-md`) with its own release cycle.
> This page documents how lean-ctx **integrates** the addon. The full `@directive`
> catalog, engine spec, and E-constructs live in the addon repo, not here.

---

## 1. What lean-md is

lean-md renders `.lmd.md` / `.lean-md` files: `@directive` calls plus a macro
engine (`@define`/`@call`), container gating (`@if`/`@consumer`), and pipes
(`@render`). Code-intel directives (`@read`/`@refactor`/`@search`/…) call lean-ctx
`ctx_*` tools **over the wire** (CLI/MCP); the renderer itself is standalone
(`rushdown` + `evalexpr`) with **no** lean-ctx crate dependency.

Engine, full directive catalog, and spec: **https://github.com/dasTholo/lean-md**.

## 2. Installation

```bash
lean-ctx addon add lean-md                 # from the bundled registry
lean-ctx addon add ./lean-ctx-addon.toml   # local manifest (dev/test)
```

After install, restart the MCP client so the gateway catalog is re-read. The addon
is spawned as a stdio gateway child; its tools (`ctx_md_render`, `ctx_md_check`)
become reachable through the lean-ctx server.

## 3. Integration points in lean-ctx

lean-ctx keeps exactly three lmd-aware touch points; everything else is the addon's.

### 3.1 Auto-render delegation hook

`ctx_read` recognizes a `.lmd.md` path and delegates rendering to the lean-md addon
via the gateway when installed; without the addon it returns the **raw** bytes
(never a half-rendered body). This is the only lmd knowledge remaining in
`rust/src` and is opt-in (fires only on `.lmd.md`). It replaces the former in-tree
`extension_registry::RenderTransform` coupling.

Source: `rust/src/tools/registered/ctx_read.rs`.

### 3.2 Addon registry entry

`rust/data/addon_registry.json` carries the installable `lean-md` entry
(`transport=stdio`, `command=lean-md`, `args=["mcp"]`, tier *community*). The
validator (`core::addons::registry::validate_entries`) requires
author/homepage/license/description and a finding-free wiring.

### 3.3 ctx_* outbound surface = addon contract

Every lean-md code-intel directive calls back into lean-ctx via
`backend.call("ctx_*", …)`. That tool set (`ctx_read`, `ctx_refactor`,
`ctx_search`, `ctx_outline`, `ctx_impact`, `ctx_repomap`, `ctx_review`,
`ctx_routes`, `ctx_smells`, `ctx_architecture`, `ctx_graph`, `ctx_callgraph`,
`ctx_knowledge`, `ctx_handoff`, `ctx_agent`, …) is a stable **outbound contract**
and must stay registered. Only `ctx_md_render` / `ctx_md_check` are addon-provided
and absent from lean-ctx.

## 4. Decoupling rationale (vs. main)

lean-md was developed in-tree (phases 1–9) and then **reverse-cut** before merge:
the in-tree engine never reaches `main`. The lmd-related deltas this branch lands
in lean-ctx are integration-only.

| Class   | Change (vs. main)                                                       | Why                                                |
|---------|-------------------------------------------------------------------------|--------------------------------------------------|
| added   | auto-render delegation hook in `ctx_read.rs`                            | render `.lmd.md` via the addon, no in-tree engine  |
| changed | `addon_registry.json`: `lmd` placeholder → installable `lean-md` entry  | installable via `lean-ctx addon add`               |
| kept    | `extension_registry::RenderTransform` trait + `WasmRenderTransform`     | generic infra, not lmd-exclusive                   |
| kept    | ctx_* outbound tool surface                                             | the addon calls them over the wire                 |
| added   | gate tests `reverse_cut_gate.rs`, `auto_render_delegation.rs`           | enforce the cut invariant + raw-fallback           |

The engine, full `@directive` catalog, E-constructs, and spec now live in
`dasTholo/lean-md` and are **not** mirrored here.

## 5. See also

- Addon repo (engine + full directive reference): https://github.com/dasTholo/lean-md
- Addon manifest contract: `docs/contracts/addon-manifest-v1.md` (upstream)
- MCP tool catalog: [`appendix-mcp-tools.md`](appendix-mcp-tools.md)
- Decoupling design: https://github.com/dasTholo/lean-md (addon repo — hosts engine, spec & decoupling design)
