# Faithful R2 lean-ctx arm

The R2 "Who Owns the Context Window?" round (Entelligentsia / tokbench, on the
Forge/forge-cli + pi runtime) is an oracle-free **planted-bug fix** task judged by
rebuild + reproduce. This directory holds the lean-ctx arm config so that
**"installed = running as designed"** — the R1 round ran lean-ctx with its
overhead defaults on, which cost a per-turn injected-prefix tax.

## What the faithful arm changes (vs R1 defaults)

| Lever | Setting | Why it matters on a phase-isolated harness |
|------|---------|--------------------------------------------|
| Zero injection | `rules_injection = off` | Drops the rule-file half of the ~3K-token per-turn prefix that R1 re-billed every turn. |
| Minimal surface | `minimal_overhead = true`, `tool_profile = minimal` | Drops the tool-schema half of that prefix (6-tool core, not the full surface). |
| Cold reads | `structure_first = true` | Biases `auto` → `map` for medium source files on a cold read (the only read saving that survives a fresh process), while capability guards keep suspect files full. |
| Shell routing | pi `mode = replace` / `routeShell = true` | Forces build/test/make output through `ctx_shell` (R1 saw 102 native bash / 0 ctx_shell — uncompressed). |
| Surface reach | `proxy_enabled = true`, `history_mode = cache-aware` | The proxy compresses the *whole* request body (incl. `forge_*` store output and native shell), with a byte-stable prefix so a cached rail keeps hitting. |

These are the three dominance vectors: **capability** (localize the defect in
fewer turns, never compress the suspect away), **surface** (proxy + shell), and
**honesty** (`lean-ctx gain` reports net-of-injection — see `meter-honest`).

## Files

- `lean-ctx.toml` — engine config. Copy to `$XDG_CONFIG_HOME/lean-ctx/config.toml`, or drop into the repo workspace as `.lean-ctx.toml`.
- `faithful-arm.env` — the same settings as env vars, plus the proxy base-URL wiring. Source it for harnesses that prefer env over files.
- `pi-config.json` — pi extension config (`~/.pi/agent/extensions/pi-lean-ctx/config.json`) for the pi/forge runtime.

## Run it (pi / forge-cli runtime — the R2 rail)

```bash
# 1. install config
mkdir -p ~/.pi/agent/extensions/pi-lean-ctx
cp bench/agent-task/r2/pi-config.json ~/.pi/agent/extensions/pi-lean-ctx/config.json
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/lean-ctx"
cp bench/agent-task/r2/lean-ctx.toml "${XDG_CONFIG_HOME:-$HOME/.config}/lean-ctx/config.toml"

# 2. start the wire-level proxy (foreground; background it for a run)
lean-ctx proxy start --port=4444 &

# 3. point the agent at the proxy
set -a; source bench/agent-task/r2/faithful-arm.env; set +a
```

## Run it (this repo's Claude harness)

`bench/agent-task` wires the `leanctx` arm purely via `lean-ctx init`
(`swebench_harness/run_arm.py`). To run it faithfully, apply the engine config
and proxy into the arm's fresh `HOME` and source `faithful-arm.env` before the
agent launches — the harness's own protocol stays unchanged.

## Verify the arm is actually faithful

```bash
lean-ctx config get rules_injection   # -> off
lean-ctx config get tool_profile      # -> minimal
lean-ctx proxy status                 # -> running on :4444, compression stats
lean-ctx gain                         # -> net_tokens_saved (net of injected overhead)
```

## tokbench PR offer

devasur invited patches on #361 ("we would appreciate your patch offer if you
can send us a PR"). The integration PR to tokbench is exactly this arm:

1. add the lean-ctx arm using `pi-config.json` + `lean-ctx.toml` above,
2. start `lean-ctx proxy start` for the arm and export `faithful-arm.env`,
3. ship the pi-extension `routeShell` fix (this repo, `packages/pi-lean-ctx`) so
   shell output reaches the compressor without the agent having to choose
   `ctx_shell`.

Right-of-reply framing: lean-ctx is the only **code-aware** arm (localizes +
compresses without hiding the defect), the **broadest reach** via the proxy, and
the **only meter that reconciles to the provider bill**. rtk is shell-only and
architecturally capped; headroom is a blind wire compressor that under-compresses
code/prose by default and can compress bug-relevant content away.
