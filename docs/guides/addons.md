# Addons — community extensions for lean-ctx

Addons let anyone extend lean-ctx with an **external MCP server** and have it
show up through the gateway with one command — no fork, no recompile. This guide
covers using addons and **building & publishing your own**.

Contract: [`addon-manifest-v1`](../contracts/addon-manifest-v1.md).

## Use an addon

```bash
lean-ctx addon list               # installed addons + the registry
lean-ctx addon search markdown    # search the registry (empty = list all)
lean-ctx addon info <name>        # details + the MCP wiring it would add
lean-ctx addon add <name>         # install (asks for confirmation)
lean-ctx addon remove <name>      # uninstall
```

`add` prints the exact server it will run (transport, command, args, env) and
asks before changing anything. Pass `--yes` / `-y` to skip the prompt in
scripts. Installing an addon enables the MCP gateway (`gateway.enabled = true`);
its tools become reachable via `ctx_tools` (find/call) — restart your MCP client
to pick them up.

## Build your own addon

An addon is just an MCP server plus a manifest. Four steps:

### 1. Expose your tool as an MCP server

Ship a `stdio` server (an executable that speaks MCP over stdin/stdout) or an
`http` server (a streamable-HTTP endpoint). This is what lean-ctx will run or
connect to. If your project is currently a library or a fork, wrap its
capabilities behind a thin MCP server binary — that is what makes it a runtime
addon instead of a build-time fork.

### 2. Add `lean-ctx-addon.toml` to your repo

```toml
[addon]
name = "my-addon"                 # slug: [a-z0-9-]
display_name = "My Addon"
version = "0.1.0"
description = "What it does, in one line."
author = "you"
homepage = "https://github.com/you/my-addon"
license = "Apache-2.0"
categories = ["workflow"]
keywords = ["plans", "macros"]
min_lean_ctx = "3.8.0"

[mcp]
transport = "stdio"               # or "http"
command = "my-addon-mcp"          # stdio: executable to spawn
args = ["serve"]
# env = { MY_TOKEN = "..." }      # optional child-process env

# For an HTTP server instead of stdio:
# [mcp]
# transport = "http"
# url = "https://my-addon.example.com/mcp"
# headers = { Authorization = "Bearer ..." }
```

See the [contract](../contracts/addon-manifest-v1.md) for every field.

### 3. Test it live — locally, before publishing

```bash
lean-ctx addon add ./lean-ctx-addon.toml
lean-ctx addon list               # your addon, installed (source: local)
# … exercise it via ctx_tools …
lean-ctx addon remove my-addon
```

`addon add <path>` wires a local manifest exactly like a registry entry, so you
get the full install flow without touching the registry.

### 4. Get listed in the registry

Open a merge request adding your manifest as an entry to
`rust/data/addon_registry.json`:

```json
{
  "addon": {
    "name": "my-addon",
    "display_name": "My Addon",
    "description": "What it does, in one line.",
    "author": "you",
    "homepage": "https://github.com/you/my-addon",
    "license": "Apache-2.0",
    "categories": ["workflow"],
    "keywords": ["plans", "macros"],
    "min_lean_ctx": "3.8.0"
  },
  "mcp": {
    "transport": "stdio",
    "command": "my-addon-mcp",
    "args": ["serve"]
  }
}
```

Once merged, everyone can run `lean-ctx addon add my-addon`, and your addon
appears on the website's Addons page.

> **Not ready to publish an endpoint yet?** Submit a *listed* entry — the
> `[addon]` table without an `[mcp]` block. It shows up in the registry and on
> the website and links to your homepage; `addon add` points users there until
> you ship the endpoint, then adding the `mcp` block flips it to one-click
> installable.

## How it works

- Installing writes a `[[gateway.servers]]` entry to your global `config.toml`
  and records the addon in `<data_dir>/addons/installed.json`. The gateway is
  **global-only** and opt-in — an untrusted project can never wire a server.
- `remove` drops exactly the gateway server the addon installed. It leaves the
  gateway enabled; turn it off with `lean-ctx config set gateway.enabled false`.
- Everything is local and deterministic: no network calls or telemetry in the
  add/list/search/info/remove paths.

## Security & trust

An addon runs real code with your privileges (stdio) or sends context to a remote
endpoint (http), so lean-ctx makes installing one a disclosed, policy-gated
action. Full model: the [contract](../contracts/addon-manifest-v1.md#security-model).

- **Trust tier.** Catalog entries are **verified** (maintainer-audited) or
  **community** (installable, unaudited). The tier shows in `addon list`,
  `addon info` and the install preview.
- **Risk review.** Before install, lean-ctx prints a security review of the
  wiring — remote endpoints, shelling out, unpinned upstreams, secret-bearing env
  — so you see what an addon can do before you say yes.
- **Untrusted output.** An addon's tool output is redacted for secrets and
  audit-tagged as untrusted before it reaches the model.

### Lock it down (teams / enterprise)

The global-only `[addons]` block sets a floor an untrusted repo can't loosen:

```bash
# only install maintainer-verified addons
lean-ctx config set addons.policy verified_only

# or restrict to an explicit allowlist
lean-ctx config set addons.policy allowlist
lean-ctx config set addons.allowlist my-addon,other-addon

# refuse anything with a high-risk capability
lean-ctx config set addons.block_risky true

# sandbox spawned addon servers (macOS sandbox-exec / Linux bwrap)
lean-ctx config set addons.sandbox strict

# require a signed user-override registry (trusted org key)
lean-ctx config set addons.require_signature true

lean-ctx config schema addons   # inspect every key
```

Distribute these via MDM / config-management, or pin them through the signed
org-policy floor (`policy org`) to make them un-bypassable.

## Troubleshooting

```bash
lean-ctx addon list               # is it installed? which gateway server?
lean-ctx config schema gateway    # inspect gateway config keys
lean-ctx status                   # MCP server / gateway status
```

If a freshly installed addon's tools do not appear, restart your MCP client so
it re-reads the gateway catalog.
