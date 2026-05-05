# pi-lean-ctx

CLI-first [Pi Coding Agent](https://github.com/badlogic/pi-mono) extension that routes Pi’s built-in tools through [lean-ctx](https://leanctx.com) for **60–90% token savings**.

- **Default**: CLI-only (no MCP required)
- **Optional**: enable MCP tools (`LEAN_CTX_PI_ENABLE_MCP=1`) or run `lean-ctx init --agent pi --mode mcp`

## What it does

### Built-in Tool Overrides (CLI)

Overrides Pi's built-in tools to route them through `lean-ctx`:

| Tool | Compression |
|------|------------|
| `bash` | All shell commands compressed via lean-ctx's 95+ patterns |
| `read` | Smart mode selection (full/map/signatures) based on file type and size |
| `grep` | Results grouped and compressed via ripgrep + lean-ctx |
| `find` | File listings compressed and .gitignore-aware |
| `ls` | Directory output compressed |

### Direct lean-ctx CLI tool

The extension registers a `lean_ctx` tool that runs `lean-ctx` directly (no nested compression).
Use it for commands like:

- `lean-ctx overview`
- `lean-ctx session …`
- `lean-ctx knowledge …`
- `lean-ctx gain` / `lean-ctx stats`
- `lean-ctx index …`

### Optional MCP Tools (Embedded Bridge)

By default, pi-lean-ctx does **not** start an MCP server. If enabled, it spawns `lean-ctx` as an MCP
server and registers advanced tools directly in Pi:

| Tool | Purpose |
|------|---------|
| `ctx_session` | Session state management and persistence |
| `ctx_knowledge` | Project knowledge graph with temporal validity |
| `ctx_semantic_search` | Find code by meaning, not exact text |
| `ctx_overview` | Codebase overview and architecture analysis |
| `ctx_compress` | Manual compression control |
| `ctx_metrics` | Token savings dashboard |
| `ctx_agent` | Multi-agent coordination and handoffs |
| `ctx_graph` | Dependency graph analysis |
| `ctx_discover` | Smart code discovery |
| `ctx_context` | Context window management |
| `ctx_preload` | Predictive file preloading |
| `ctx_delta` | Changed-lines-only reads |
| `ctx_edit` | Read-modify-write in one call |
| `ctx_dedup` | Duplicate context elimination |
| `ctx_fill` | Template completion |
| `ctx_intent` | Intent-based task routing |
| `ctx_response` | Response optimization |
| `ctx_wrapped` | Wrapped command execution |
| `ctx_benchmark` | Compression benchmarking |
| `ctx_analyze` | Code analysis |
| `ctx_cache` | Cache management |
| `ctx_execute` | Direct command execution |

If you don’t want MCP: keep it disabled and use the CLI overrides + `lean_ctx` tool only.

## Install

```bash
# 1. Install lean-ctx (if not already installed)
cargo install lean-ctx
# or: brew tap yvgude/lean-ctx && brew install lean-ctx

# 2. Install the Pi package
pi install npm:pi-lean-ctx

# 3. Restart Pi
```

Or use the automated setup:

```bash
lean-ctx init --agent pi
```

## How it works

### CLI overrides (bash, read, grep, find, ls)

These tools invoke the `lean-ctx` binary via CLI with `LEAN_CTX_COMPRESS=1`. The output is parsed for compression stats and displayed with a token savings footer.

### Optional MCP bridge (all other tools)

If you enable the MCP bridge, pi-lean-ctx spawns the `lean-ctx` binary as an MCP server (JSON-RPC over stdio).
It discovers available tools via `list_tools`, filters out those already covered by CLI overrides, and registers the rest as native Pi tools.

If `lean-ctx` is already configured as an MCP server via [pi-mcp-adapter](https://github.com/nicobailon/pi-mcp-adapter) in `~/.pi/agent/mcp.json`, the embedded bridge is skipped to avoid duplicate tools.

### Automatic reconnection

If the MCP server process crashes, the bridge automatically reconnects (up to 3 attempts with exponential backoff). If reconnection fails, CLI-based tools continue working normally — only the advanced MCP tools become unavailable.

## Enabling MCP (optional)

Set an environment variable and restart Pi:

```bash
export LEAN_CTX_PI_ENABLE_MCP=1
pi
```

Or configure MCP via `lean-ctx init`:

```bash
lean-ctx init --agent pi --mode mcp
```

## pi-mcp-adapter compatibility

If you prefer using [pi-mcp-adapter](https://github.com/nicobailon/pi-mcp-adapter) to manage your MCP servers, lean-ctx integrates automatically:

```bash
# Option A: lean-ctx writes the config for you
lean-ctx init --agent pi

# Option B: Manual configuration in ~/.pi/agent/mcp.json
```

```json
{
  "mcpServers": {
    "lean-ctx": {
      "command": "/path/to/lean-ctx",
      "lifecycle": "lazy",
      "directTools": true
    }
  }
}
```

When pi-mcp-adapter manages the lean-ctx MCP server, pi-lean-ctx detects this and only registers its CLI-based tool overrides, leaving MCP tool management to the adapter.

## Binary Resolution

The extension locates the `lean-ctx` binary in this order:

1. `LEAN_CTX_BIN` environment variable
2. `~/.cargo/bin/lean-ctx`
3. `~/.local/bin/lean-ctx` (Linux) or `%APPDATA%\Local\lean-ctx\lean-ctx.exe` (Windows)
4. `/usr/local/bin/lean-ctx` (macOS/Linux)
5. `lean-ctx` on PATH

## Smart Read Modes

The `read` tool automatically selects the optimal lean-ctx mode:

| File Type | Size | Mode |
|-----------|------|------|
| `.md`, `.json`, `.toml`, `.yaml`, etc. | Any | `full` |
| Code files (55+ extensions) | < 8 KB | `full` |
| Code files | 8–96 KB | `map` (deps + API signatures) |
| Code files | > 96 KB | `signatures` (AST extraction) |
| Other files | < 48 KB | `full` |
| Other files | > 48 KB | `map` |

## Slash Command

Use `/lean-ctx` in Pi to check:
- Which binary is being used
- MCP bridge status (disabled / embedded / adapter)
- Number and names of registered MCP tools

## Disabling specific tools

To disable specific MCP tools, configure `disabled_tools` in `~/.lean-ctx/config.toml`:

```toml
disabled_tools = ["ctx_graph", "ctx_benchmark"]
```

Or via environment variable:

```bash
LEAN_CTX_DISABLED_TOOLS=ctx_graph,ctx_benchmark pi
```

## Links

- [lean-ctx](https://leanctx.com) — The Cognitive Filter for AI Engineering
- [GitHub](https://github.com/yvgude/lean-ctx)
- [Discord](https://discord.gg/pTHkG9Hew9)
