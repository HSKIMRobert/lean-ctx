# Journey 12 — The Troubleshooting Playbook

> Something isn't working: your AI doesn't seem to use lean-ctx, savings are
> zero, recall broke, or a command behaves oddly. This is the **central**
> playbook — symptom → one-line diagnosis → fix — that ties together the repair
> tools scattered across the other journeys.

Source files:
- `rust/src/doctor/` — `doctor`, `doctor integrations`, `doctor --fix`
- `rust/src/cli/sessions_doctor.rs` — `sessions doctor`
- `rust/src/hooks/mod.rs` — hook install/refresh
- `rust/src/core/updater.rs` — `post_update_rewire`

---

## 0. The 30-second triage

Run these three, in order. Each one's footer tells you the next step:

```bash
lean-ctx status                 # is the wiring there at all?  (5-line summary)
lean-ctx doctor                 # ~27 checks across binary/daemon/proxy/caches
lean-ctx doctor integrations    # per-IDE: MCP + hook freshness + rules, per agent
```

`status` is the fast yes/no, `doctor` is the deep scan, and `doctor
integrations` pinpoints *which editor* is mis-wired. Most problems below are
identified by one of these three and fixed by `lean-ctx setup --fix`.

---

## 1. "My AI isn't using lean-ctx at all"

**Diagnose:** `lean-ctx doctor integrations` — find the agent you're using and
read its line.

| What you see | Meaning | Fix |
|--------------|---------|-----|
| Agent not listed | lean-ctx didn't detect it | `lean-ctx init --agent <name>` |
| `MCP config … missing` / `drift` | server not wired | `lean-ctx setup --fix` |
| `Hooks … drift` | shell hook missing/incomplete | `lean-ctx setup --fix` |
| `Hooks … stale binary …` | hook points at an old install path | `lean-ctx setup --fix` |
| All `✓` but still nothing | the **editor wasn't restarted** | fully quit & reopen the editor |

The last row is the most common: editors load MCP servers and hooks at startup,
so a config written after launch only takes effect on the next restart.

---

## 1b. "The CLI and my editor (MCP) read different config"

**Symptom:** a setting applied in the terminal (`lean-ctx config set …`) is
ignored by the MCP server inside your editor — e.g. a custom `path_jail` works on
the CLI but not in-editor.

**Cause:** an older lean-ctx baked `LEAN_CTX_DATA_DIR` into the editor's MCP
server `env`. That forced the server into single-dir mode, so it read
`config.toml` from the **data** dir (`~/.local/share/lean-ctx`) while the CLI
read it from the **config** dir (`~/.config/lean-ctx`).

**Diagnose:** `lean-ctx doctor` flags `config location — stray config.toml in the
data dir` when this happens.

**Fix:** `lean-ctx doctor --fix` (or just `lean-ctx update` / `lean-ctx setup`).
It strips the stale env from every editor config and **losslessly** relocates a
stray data-dir `config.toml` into the canonical config dir, so both read the same
file again. Restart the editor afterwards.

> Current versions never pin `LEAN_CTX_DATA_DIR` in MCP configs, and a data-dir
> pin at the *standard* location is treated as data-only — so config no longer
> diverges even if a stale env lingers from an older install.

---

## 2. "`gain` shows zero / savings look wrong"

**Diagnose:** is anything routed through lean-ctx yet?

- A brand-new install legitimately shows *"No savings recorded yet — and that's
  expected."* Savings accrue only as the `ctx_*` tools and shell hook are used.
- If you've been working but `gain` is still empty, your terminal commands aren't
  being intercepted. Run `lean-ctx ghost` (hidden waste from uncompressed
  commands) and `lean-ctx discover` (missed-compression opportunities in your
  shell history) to confirm, then re-check the hook with `doctor integrations`.

`gain` and `token-report` read from the same stats store; if one shows numbers
and the other doesn't, you're looking at savings vs. memory footprint — that's
expected (see [Journey 11](11-analytics-and-insights.md)).

---

## 3. "A new chat doesn't remember where we were"

Session auto-restore is failing. There's a dedicated repair tool:

```bash
lean-ctx sessions doctor          # diagnose session-restore health
lean-ctx sessions doctor --fix    # repair the latest-pointer / snapshots
```

Common causes: the project root changed (sessions are project-scoped), or
`sessions/latest.json` got out of sync. `sessions doctor --fix` rebuilds the
pointer. See [Journey 3 → Auto-restore](03-memory-and-knowledge.md) for the
`ACTIVE SESSION` block this restores.

---

## 4. "Native Read/Grep are being denied"

This is **harden mode**, not a bug. If you (or a teammate) ran `lean-ctx harden`,
native file tools are intentionally denied so the agent uses the compressed
`ctx_*` tools. Turn it off with:

```bash
lean-ctx harden --undo            # native tools allowed again
```

See [Journey 13 → Harden](13-security-and-governance.md) for what each level does.

---

## 5. "My shell is broken after install"

The shell hook or proxy modified your RC file. lean-ctx always keeps a backup:

```bash
lean-ctx doctor --fix             # re-runs the safe, merge-based wiring
lean-ctx proxy status             # is a *_BASE_URL export pointing at the proxy?
```

Every RC edit is preserved as a `*.lean-ctx.bak` sibling. If a base URL
"defaults to the wrong provider," check the exported `*_BASE_URL` values in your
RC and `lean-ctx proxy disable` to remove them. The emergency, no-binary fallback
is in [Journey 6 → Emergency](06-lifecycle.md).

---

## 6. "Search/indexing seems stuck or huge"

```bash
lean-ctx index status             # is each index ready + recent?
lean-ctx cache prune              # drop oversized/quarantined/orphaned indexes
```

If `index status` shows a very old build time, the watcher isn't running —
`lean-ctx index watch` (or just `setup --fix`) restarts it. If the BM25 index is
quarantined, `cache prune` removes it and the next read rebuilds it. To bound
index size proactively, see [Journey 14 → Performance](14-performance-tuning.md).

---

## 7. "After `lean-ctx update`, an editor stopped working"

`update` runs `post_update_rewire`, which refreshes every installed shell-hook
agent so hooks point at the *new* binary. If one agent slipped through:

```bash
lean-ctx doctor integrations      # look for `stale binary` on the affected agent
lean-ctx setup --fix              # re-point all hooks at the current binary
```

The set of auto-refreshed agents is registry-driven (`refresh_installed_hooks`);
MCP-only agents need no hook refresh because they always exec the current binary.

---

## 7b. "Where did `ctx_edit` go? My agent has no edit tool"

Not a bug — a deliberate redesign of the editing story:

- **`ctx_edit` (str_replace) is power-only** since v3.8.12. In editors with a
  reliable native edit tool (Cursor, Zed, Windsurf, …), a second search-and-replace
  editor added schema tokens to every session without saving any.
- **`ctx_patch` (anchored editing) is the successor** and part of the lazy core
  and `standard` profile. It edits by `line + hash` anchor from
  `ctx_read(mode="anchored")` or `ctx_search(anchored=true)` — the agent never
  reproduces old text byte-for-byte, which is where str_replace burns output
  tokens. `op=create` writes new files; batches (`ops:[…]`) apply all-or-nothing.
- **Client-aware advertising**: clients with a trusted native editor don't see
  `ctx_patch` in the default (lazy) surface — their sessions pay zero extra
  schema tokens and edits stay native. Claude Code, SDK/headless harnesses and
  unknown clients get `ctx_patch` advertised.

Both editors always stay reachable:

```bash
lean-ctx tools standard           # pin 16 tools incl. ctx_patch (client-agnostic)
lean-ctx tools power              # everything incl. ctx_edit
```

or per call via `ctx_call(name="ctx_edit", args={…})` — no profile change needed.
If `prefer_native_editor = true` is set, *both* edit tools are hidden and refused
by design (#454).

---

## 8. When all else fails — capture a report

```bash
lean-ctx report-issue             # collects a redacted diagnostic bundle
```

This gathers `doctor` output, versions, and config (secrets redacted) so a bug
report is actionable. Pair it with the exact command and the editor you used.

---

## Decision guide

| Symptom | Start here |
|---------|-----------|
| Agent ignores lean-ctx | §1 → `doctor integrations` |
| Zero/odd savings | §2 → `ghost` / `discover` |
| New chat has no memory | §3 → `sessions doctor --fix` |
| Read/Grep denied | §4 → `harden --undo` |
| Shell/proxy broken | §5 → `doctor --fix` / `proxy status` |
| Search stuck/huge | §6 → `index status` / `cache prune` |
| Broke after update | §7 → `doctor integrations` / `setup --fix` |
| Missing edit tool (`ctx_edit`) | §7b → `ctx_patch` / `lean-ctx tools power` |
| Need to file a bug | §8 → `report-issue` |
