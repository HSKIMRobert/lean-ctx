# Addon Manifest — v1

Status: **stable (v1)** · Module: `core::addons` · CLI: `lean-ctx addon`

An **addon** packages an external MCP server (plus metadata) behind a small
`lean-ctx-addon.toml` manifest, so a third-party tool plugs into lean-ctx's MCP
gateway with one `lean-ctx addon add` — no fork, no recompile. Addons are
user-global and reuse the gateway trust model: `[gateway]` is global-only (never
merged from an untrusted project-local config) and a full no-op until enabled.

This contract defines the manifest shape, the registry shape, and the install
semantics. The how-to lives in [`docs/guides/addons.md`](../guides/addons.md).

## Manifest: `lean-ctx-addon.toml`

Two tables: `[addon]` (metadata) and `[mcp]` (how lean-ctx runs the server).

### `[addon]`

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `name` | string | — (required) | Stable slug `[a-z0-9-]` (no leading/trailing dash). Becomes the gateway server name. |
| `display_name` | string | `""` | Human-friendly name (falls back to `name`). |
| `version` | string | `""` | Author-declared version (free-form). |
| `description` | string | `""` | One-line summary shown in `addon list` and on the website. |
| `author` | string | `""` | Maintainer or org. |
| `homepage` | string | `""` | Project homepage / repository URL. |
| `license` | string | `""` | SPDX id (e.g. `Apache-2.0`). |
| `categories` | string[] | `[]` | Coarse buckets for browsing (e.g. `plans`, `workflow`, `search`). |
| `keywords` | string[] | `[]` | Free-form search terms. |
| `min_lean_ctx` | string | `""` | Minimum lean-ctx version targeted (informational). |
| `verified` | bool | `false` | **Registry-controlled** trust tier. `true` only for entries a maintainer has audited and vouched for. Setting it in a hand-written manifest is meaningless — trust is conferred by the registry an entry ships in, not by the entry claiming it. |

### `[mcp]`

Mirrors a `[[gateway.servers]]` entry — installation is a direct translation.

| Field | Type | Default | Transport | Meaning |
|-------|------|---------|-----------|---------|
| `transport` | `stdio` \| `http` | `stdio` | both | Wire protocol. |
| `command` | string | `""` | stdio | Executable to spawn. |
| `args` | string[] | `[]` | stdio | Arguments passed to `command`. |
| `env` | table | `{}` | stdio | Extra environment variables for the child process. |
| `url` | string | `""` | http | Streamable-HTTP endpoint (must be `http(s)://`). |
| `headers` | table | `{}` | http | Extra request headers (e.g. auth). |

### Installable vs. listed

- **Installable** — the `[mcp]` block resolves: `stdio` has a non-empty
  `command`, or `http` has an `http(s)` `url`. `lean-ctx addon add` wires it.
- **Listed** — a registry entry **without** a runnable `[mcp]` block. It appears
  in `addon list` / `search` / the website and links to its homepage, but
  `addon add` refuses (no fabricated wiring). Used for announced addons that have
  not published an MCP endpoint yet.

## Registry

The curated catalog. Layered like the model registry:

1. **Bundled** — `rust/data/addon_registry.json`, compiled into the binary.
2. **User override** — `<data_dir>/addon_registry.json` (optional). An entry with
   the same `name` replaces the bundled one.

Shape:

```json
{
  "registry_version": 1,
  "addons": [
    { "addon": { "name": "…", "description": "…", … }, "mcp": { … } }
  ]
}
```

Each array element is exactly one manifest (the `[mcp]` table may be omitted for
listed-only entries). Getting listed = a merge request adding an entry here.

## Install semantics

`lean-ctx addon add <name|path>`:

1. **Resolve** the manifest — by registry `name`, or from a local
   `lean-ctx-addon.toml` path (a path ends in `.toml`, contains `/`, starts with
   `.`, or is an existing file).
2. **Validate** metadata; require an installable `[mcp]` block (else refuse with
   a homepage pointer).
3. **Assess + disclose** — statically review the `[mcp]` wiring for risk signals
   (remote endpoint, shelling out, unpinned upstream, secret-bearing env), print
   the trust tier, the exact transport/command/args/env (or url/headers), and any
   findings.
4. **Gate** — enforce the global-only `[addons]` install policy (see below).
   A blocked addon never reaches the next step.
5. **Confirm** — require confirmation (`--yes`/`-y` to skip; refuses
   non-interactively without it, per [`cli::prompt`]).
6. **Wire** via `Config::update_global` (the safe, global-only persistence path):
   set `gateway.enabled = true` if it was off, then upsert a `[[gateway.servers]]`
   entry named after the addon (idempotent — replaces any same-named entry).
7. **Record** in `<data_dir>/addons/installed.json` (`name`, `version`, `source`,
   `gateway_server`) and invalidate the gateway catalog cache.

`lean-ctx addon remove <name>` reverses 4–5: drop the gateway server it owns and
the store entry. It leaves `gateway.enabled` untouched (disable explicitly with
`lean-ctx config set gateway.enabled false`).

### State vs. config

The live `[[gateway.servers]]` block in `config.toml` is the single source of
truth for what actually runs. `installed.json` is bookkeeping only — it maps an
addon to the gateway server it installed so `remove` unwinds exactly what `add`
wired. Deleting it never affects running servers.

## Security model

An addon is **executable trust**: a `stdio` addon spawns a child process with
your privileges; an `http` addon sends context to a remote endpoint; and every
addon's tool output flows into the model context (a prompt-injection surface). An
addon is as powerful as a VS Code extension or an npm package, so lean-ctx treats
installing one as a consequential, disclosed, policy-gated action.

### Baseline (always on)

- The gateway is **global-only** and **opt-in**; a project-local config can never
  point it at arbitrary commands.
- `add`/`remove` are consequential writes: they disclose the wiring and require
  confirmation — never silent.
- The bundled registry is **curated** and compiled into the binary (no live
  fetch). `addon add <path>` on a local manifest is explicit and operator-driven.
- Output is deterministic and local-only: no network calls, no telemetry in the
  add/list/search/info/remove paths.

### Trust tier

`addon.verified` splits the catalog into **verified** (maintainer-audited) and
**community** (installable, unaudited). The tier is shown in `addon list`,
`addon info` and the install preview, and on the website. It is set by the
registry, never self-asserted (see the field table).

### Static risk assessment

Before install, `core::addons::trust::assess` inspects the `[mcp]` wiring and
surfaces findings at three severities:

| Severity | Examples |
|----------|----------|
| `danger` | HTTP/remote endpoint, non-HTTPS url, inline shell (`sh -c`), fetch-and-exec (`curl`) |
| `warn` | shell metacharacters in args, unpinned package runner (`npx`/`uvx` without a version), `latest` tag |
| `info` | passes environment variables / request headers |

The same function backs the **registry CI validator**
(`core::addons::registry::validate_entries`): every bundled entry must have a
unique slug, installable entries need author/homepage/license/description and
must not shell out, fetch-and-exec, use a non-HTTPS endpoint or pull an unpinned
upstream, and **verified** entries must be free of any `warn`/`danger` finding.

### Install policy floor — `[addons]`

A **global-only** config block (never merged from a project-local file; ship it
via MDM / config-management or pin it through the signed org-policy floor). Fully
permissive by default.

| Key | Type | Default | Meaning |
|-----|------|---------|---------|
| `policy` | `open` \| `verified_only` \| `allowlist` \| `locked` | `open` | What may be installed. `verified_only` requires the verified tier; `allowlist` restricts to `addons.allowlist`; `locked` disables installs. |
| `allowlist` | string[] | `[]` | Permitted slugs when `policy = allowlist`. |
| `require_signature` | bool | `false` | Honour a user-override registry only if signed by a trusted org key. |
| `sandbox` | `off` \| `auto` \| `strict` | `off` | Sandbox spawned stdio servers (see below). |
| `block_risky` | bool | `false` | Refuse to install an addon that has a `danger` finding. |

`core::addons::policy::gate` enforces this in `install` before any gateway
mutation, so a blocked addon never touches `config.toml`.

### Registry signing

The bundled registry is trusted by construction. The risk surface is a
**user-override** registry (`<data_dir>/addon_registry.json`), which can shadow
trusted names. With `require_signature = true`, the override is honoured only if a
sidecar `addon_registry.json.sig` carries a valid Ed25519 signature **by a
trusted org key** — the same pinned-key anchor as the signed org-policy floor
(`policy org trust`). An unsigned/invalid/untrusted override is ignored (warned),
falling back to the bundled catalog.

### Sandboxing

With `addons.sandbox = auto|strict`, lean-ctx wraps each spawned stdio server in
an OS-native sandbox at the single spawn point (`core::gateway::client`):
`sandbox-exec` (macOS) or `bwrap` (Linux). Local tools rarely need the network,
so the default control is **outbound-network isolation** (`auto`); `strict` also
makes the filesystem read-only except a scratch tmp and **refuses to spawn** if
no launcher exists. Off by default — zero behavioural change unless enabled.

### Runtime redaction + audit

Downstream tool output is untrusted content. Before it reaches the model,
`core::addons::runtime::scrub_output` runs it through the same secret redaction as
the shell layer and records an audit trace tagging the bytes as untrusted,
attributed to the originating server.

### Reporting a malicious addon

Open a confidential issue on the tracker or email the maintainers. We can pull an
entry from the registry (a release ships the curated catalog) and, for a
published endpoint, advise affected users to `lean-ctx addon remove <name>`.

## CLI surface

| Command | Effect |
|---------|--------|
| `lean-ctx addon list` | Installed addons + the registry. |
| `lean-ctx addon search [query]` | Search the registry (empty = all). |
| `lean-ctx addon info <name\|path>` | Details + MCP wiring for one addon. |
| `lean-ctx addon add <name\|path> [-y]` | Install (registry or local manifest). |
| `lean-ctx addon remove <name> [-y]` | Uninstall. |
