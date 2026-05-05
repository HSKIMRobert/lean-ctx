# Claim Drift Scan v1 (Website/Docs vs Code)

Date: 2026-05-02

Goal: identify **outdated / incorrect claims** on public pages and map them to **repo evidence** (code/tests/SSOT manifests).

## Pages scanned

- `https://leanctx.com/trust`
- `https://leanctx.com/how-it-works/`
- `https://leanctx.com/docs/tools/`
- `https://leanctx.com/docs/getting-started/`
- `https://leanctx.com/docs/verification`
- `https://leanctx.com/docs/replayability/`

## Drift (public claim is outdated / wrong)

### Tool count: “49 tools”

- **Claim sources**:
  - `https://leanctx.com/how-it-works/`: “Layer 1 49 intelligent tools …”
  - `https://leanctx.com/docs/tools/`: “Complete reference for all 49 …”, “LeanCTX provides 49 MCP tools …”
  - `https://leanctx.com/docs/getting-started/`: “MCP Server with 49 tools …”
- **Repo reality (SSOT)**:
  - `website/generated/mcp-tools.json` reports `counts.granular = 56` and `counts.unified = 5` (total objects across categories = 61).
  - CI gate already enforces doc/tool-count consistency in-repo; public pages are lagging.
- **Fix proposal**:
  - Remove hard-coded counts from public pages and render from SSOT manifest (or update to 56 where dynamic is not possible).
- **Status**: Fixed in CLI Tracking Premium pass — all repo files updated to 56.

### Setup auto-detect list is incomplete

- **Claim source**:
  - `https://leanctx.com/docs/getting-started/`: “setup automatically detects and configures: Cursor, Claude Code, Windsurf, VS Code / Copilot, Codex CLI, Gemini CLI, Zed, Antigravity, OpenCode, Crush, and Pi.”
- **Repo reality**:
  - `rust/src/core/editor_registry/detect.rs` includes additional first-class targets (e.g. Qwen Code, Trae, Amazon Q, JetBrains IDEs, AWS Kiro, Hermes Agent, Amp, Verdent, Roo Code).
- **Fix proposal**:
  - Replace the hard-coded list with a generated list derived from `build_targets()` (website build-time) or update the copy to “configures all detected editors” + link to compatibility table.

## OK (claim matches code)

### RRF cache eviction (K=60)

- **Claim source**: `https://leanctx.com/how-it-works/` (RRF eviction description).
- **Repo evidence**: `rust/src/core/cache.rs` implements “Reciprocal Rank Fusion eviction scores” and tests for the behavior.

### Output verification checks & warning format

- **Claim source**: `https://leanctx.com/docs/verification` (missing_path / mangled_identifier / line numbers / structure + `[VERIFY] ... loss=...%`).
- **Repo evidence**: `rust/src/core/output_verification.rs` contains those warning labels and the formatted `[VERIFY]` line.

## Missing (feature exists in code but under-claimed)

- **Universal integration autotuning**:
  - Repo: `lean-ctx doctor integrations` + docs-backed constraints matrix + instruction compiler (`lean-ctx instructions`) now exist and are CI-gated.
  - Public docs: not yet surfaced as a first-class “works across IDEs” verification story.

