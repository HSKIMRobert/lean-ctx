# Implementation Protocol — SocratiCode Case-Killer (v3.4.7)

Living document tracking all implementation work for the "SocratiCode Case-Killer" roadmap.

**Goal**: lean-ctx becomes the **local-first Context Runtime Layer** for AI Coding Agents — not just compression, but **Discovery + Impact + Artifacts + Governance**. Developers/teams don't need a separate Codebase-Index-SaaS because lean-ctx delivers the same core capabilities locally + self-hosted.

---

## Reality check (verified entrypoints)

This doc is only useful if it matches what users can actually invoke.

- **GitLab source of truth**: Primary repo is `root/lean-ctx` on `gitlab.pounce.ch`.  
  **Context‑OS Backlog** wurde (für diese Workspace‑Automation) in `pounce/pounce` (Project ID **1**) angelegt; siehe `server.md` (Token‑Scope).
- **MCP tool `ctx_pack` is real and dispatched**: `LeanCtxServer::dispatch_utility_tools` has a `\"ctx_pack\"` arm calling `crate::tools::ctx_pack::handle(...)`.
- **CLI `lean-ctx pack` and `lean-ctx index` were NOT wired previously** (so `lean-ctx pack --pr` / `lean-ctx index status` would just print global help).
  - They are now wired in code via `rust/src/cli/dispatch.rs` and implemented in `rust/src/cli/pack_cmd.rs` + `rust/src/cli/index_cmd.rs`.
  - Both were smoke-tested locally:
    - `lean-ctx pack --pr --json --depth 2 --base HEAD~1`
    - `lean-ctx index status`

**Important**: In the released `v3.4.7` binary, some entrypoints were missing (CLI wiring + tool manifest entries). The implementation is present in code, but discovery/wiring required follow-up work.

---

## Context OS — Phase 1 (Hardening + Positioning) — Execution Log

### 2026-05-02 — Kickoff: SSOT/Drift Gates + Website Nav Hardening

**Ziel**: Tool-/Docs‑Drift eliminieren und die Website‑IA in Richtung „Context OS (5 Pillars)“ umbauen.

- **SSOT Manifest Gate**:
  - `cargo run --bin gen_mcp_manifest` regeneriert `website/generated/mcp-tools.json`
  - `rust/tests/mcp_manifest_up_to_date.rs` failt jetzt hart, wenn `website/` existiert aber `mcp-tools.json` fehlt (kein stilles Skip mehr)
- **Docs Claim Drift Gate v1**:
  - Neues CI‑Gate `rust/tests/docs_tool_counts_up_to_date.rs` (Tool‑Count Claims ↔ Runtime SSOT)
- **Docs/Template Harmonisierung**:
  - Zentrale Docs/Templates/Skills/Readmes auf den aktuellen SSOT‑Tool‑Count gebracht
- **Website (deploy worktree)**:
  - Hardcoded Tool‑Count `(46)` entfernt (Header + Docs Sidebar + Docs Changelog Sidebar)
  - Count wird aus `website/generated/mcp-tools.json` gelesen
  - IA v2 umgesetzt: `contextOsIa.ts`, Header+DocsSidebar auf 5 Pillars, neue Landings (`/docs/context-os` + `/docs/context-*`) inkl. Locale‑Routen
  - Redirect‑Stubs (301) für `/docs/pillars/*` in `website/nginx.conf`
  - SSOT Tools synced: Deploy‑Manifest auf **53 Tools** aktualisiert (inkl. `ctx_call`, `ctx_pack`, `ctx_index`, `ctx_artifacts`), Tool‑Enrichments erweitert, Tool‑Pages neu generiert; `validate-manifest` prüft zusätzlich Sync `public/generated/mcp-tools.json` ↔ `generated/mcp-tools.json`; `/docs/tools` rendert Kategorien/Tools dynamisch aus SSOT.

**Evidence**:
- `cd rust && cargo test --all-features`
- `cd rust && cargo test --test mcp_manifest_up_to_date`
- `cd rust && cargo test --test docs_tool_counts_up_to_date`
- `cd website && npm run validate:manifest` (deploy worktree)

**GitLab Tickets (Context‑OS)**:
- `0.1` Pillar Navigation + Redirect Plan → closed
- `0.2` Docs SSOT Pipeline → closed
- `4.6` Docs Claim Drift Gate v1 → closed

### 2026-05-02 — Context I/O Contracts v1 (EPIC 1: Reads/Shell/Search/Edit)

**Ziel**: Deterministische, reproduzierbare I/O Outputs (gleiches Input, gleicher State, gleiche Limits) und Premium UX ohne verwirrende Stubs.

- **1.1 ctx_read Cache Correctness v1** (`#2303`):
  - mtime validation in Cache-Entries, stale detection, premium handling fuer prompt-stale Situationen
  - Relevant: `rust/src/core/cache.rs`, `rust/src/tools/ctx_read.rs`, `rust/src/server/dispatch/read_tools.rs`
- **1.2 ctx_shell Contract v1** (`#2304`):
  - `raw` und `bypass` Flags als explicit args, klare precedence gegen Env (`LEAN_CTX_RAW`, `LEAN_CTX_DISABLED`)
  - Output-Modifikatoren (savings_note, mismatch hints) werden in raw/bypass unterdrueckt, damit Output unveraendert bleibt
  - Relevant: `rust/src/server/dispatch/shell_tools.rs`, `rust/src/tool_defs/granular.rs`
- **1.3 Search I/O Determinism v1** (`#2305`):
  - `ctx_search`: deterministische File-Traversal Reihenfolge (Paths gesammelt und lexikographisch sortiert), damit `max_results` Truncation reproduzierbar ist
  - `ctx_semantic_search`: deterministische Tie-Breaks bei gleichen RRF Scores via sekundäre Keys (file_path, symbol_name, start_line, end_line)
  - BM25 Index Search: deterministische Sortierung auch bei Score-Ties (verhindert HashMap-Iteration-Order Leaks)
  - Schema: `ctx_search ignore_gitignore` Flag (default respektiert `.gitignore`)
  - Relevant: `rust/src/tools/ctx_search.rs`, `rust/src/tools/ctx_semantic_search.rs`, `rust/src/core/vector_index.rs`, `rust/src/server/dispatch/shell_tools.rs`, `rust/src/tool_defs/granular.rs`
- **1.4 ctx_edit Contract v1** (`#2306`):
  - Preimage Guards: expected_md5, expected_size, expected_mtime_ms optional; mismatch failt ohne Write
  - TOCTOU Guard: Preimage wird vor Commit erneut gelesen; bei Abweichung Abbruch (kein partial write)
  - Atomic Write: tmp file + rename; Permissions werden soweit moeglich beibehalten
  - Optional Backup + bounded redacted diff evidence (diff_max_lines, evidence Toggle)
  - Binary Safety: invalid UTF-8 wird standardmaessig abgelehnt; allow_lossy_utf8 ist explizites Opt-in
  - Relevant: `rust/src/tools/ctx_edit.rs`, `rust/src/server/dispatch/read_tools.rs`, `rust/src/tool_defs/granular.rs`
- **1.5 I/O Boundary Contract v1** (`#2307`):
  - Zentraler Boundary Helper (`rust/src/core/io_boundary.rs`) + Role IO Policy (warn|enforce, allow_secret_paths, allow_ignore_gitignore)
  - `resolve_path` routed ueber boundary + PolicyViolation Events bei Denials/Warnungen
  - `ctx_search`: secret-like files werden standardmaessig geskippt; `ignore_gitignore` nur mit expliziter Policy (admin)
  - `linkedProjects` und `artifacts` Resolution nutzt Boundary; secret-like artifacts werden fuer non-allowed roles verworfen
  - Symlink escape test in PathJail
  - Relevant: `rust/src/core/io_boundary.rs`, `rust/src/core/roles.rs`, `rust/src/tools/mod.rs`, `rust/src/tools/ctx_search.rs`, `rust/src/server/dispatch/shell_tools.rs`, `rust/src/core/pathjail.rs`

**Evidence (lokal, minimal)**:
- `cd rust && cargo test -q --test mcp_manifest_up_to_date`
- `cd rust && cargo test -q bm25_search_sorts_ties_deterministically`
- `cd rust && cargo test -q rrf_merge_hybrid_is_deterministic_on_ties`
- `cd rust && cargo test -q search_results_are_deterministically_ordered_by_path`
- `cd rust && cargo test -q ctx_edit`
- `cd worktrees/deploy-ctxos/website && npm run -s validate:manifest`
- `cd rust && cargo test -q io_boundary`
- `cd rust && cargo test -q pathjail`
- `cd rust && cargo test -q ctx_search`

**GitLab Tickets (Context‑OS)**:
- `#2303` ctx_read Cache Correctness v1 → closed
- `#2304` Shell I/O Contract v1 → closed
- `#2305` Search I/O Determinism v1 → closed
- `#2306` ctx_edit Contract v1 → closed
- `#2307` I/O Boundary Contract v1 → closed

### 2026-05-02 — Redaction + Secret Safety v1 (EPIC 4: Verification)

- **4.4 Redaction + Secret Safety v1** (`#2327`):
  - Zentral: `rust/src/core/redaction.rs` (deterministische Pattern-Redaction)
  - Policy: non-admin roles → redaction immer aktiv; admin kann via `io.redact_outputs=false` deaktivieren
  - Wiring: `ctx_read`/`ctx_search`/`ctx_shell` Outputs redacted; persisted archives werden vor Write redacted
  - CI Gate: `rust/tests/secret_scan_artifacts.rs` (scan committed generated artifacts + docs)
  - Docs: `SECURITY.md` Threat Model (v1)
  - Relevant: `rust/src/core/redaction.rs`, `rust/src/tools/ctx_read.rs`, `rust/src/tools/ctx_search.rs`, `rust/src/server/dispatch/shell_tools.rs`, `rust/src/server/mod.rs`, `rust/tests/secret_scan_artifacts.rs`, `SECURITY.md`

**Evidence (lokal, minimal)**:
- `cd rust && cargo test -q redaction`
- `cd rust && cargo test -q --test secret_scan_artifacts`
- `cd rust && cargo run -q --bin gen_mcp_manifest`
- `cd worktrees/deploy-ctxos/website && npm run -s validate:manifest`

**GitLab Tickets (Context‑OS)**:
- `#2327` Redaction + Secret Safety v1 → closed

### 2026-05-02 — Output Verification Contract v1 (EPIC 4: Verification)

- **4.1 Output Verification Contract v1** (`#2324`):
  - Config surface: `VerificationConfig.mode` (off|warn|fail) als expliziter Modus (strict_mode bleibt als Legacy-Alias kompatibel)
  - WARN/FAIL Semantik: `format_compact()` ist deterministisch (sorted keys via BTreeMap) und stabil parsebar
  - Strict mode Tests: Medium+High → FAIL; non-strict: nur High → FAIL
  - Relevant: `rust/src/core/output_verification.rs`, `rust/src/core/profiles.rs`, `rust/src/server/mod.rs`

**Evidence (lokal, minimal)**:
- `cd rust && cargo test -q output_verification`

**GitLab Tickets (Context‑OS)**:
- `#2324` Output Verification Contract v1 → closed

### 2026-05-02 — Proof Artifact Format v1 + Export Tool (EPIC 4: Verification)

- **4.2 Proof Artifact Format v1** (`#2325`):
  - JSON Schema: `ContextProofV1` (`rust/src/core/context_proof.rs`) — verifier snapshot + SLO snapshot + pipeline stats + context ledger summary + bounded tool receipts
  - Export tool:
    - **MCP**: `ctx_proof action=export ...` (dispatch via `rust/src/server/dispatch/utility_tools.rs`)
    - **CLI**: `lean-ctx proof ...` (`rust/src/cli/proof_cmd.rs`)
  - Writes: `.lean-ctx/proofs/context-proof-v1_*.json` (atomic write; proof output is always redacted)
  - Website (deploy): tool pages + enrichments + manifest validation updated (56 tools)
  - Relevant: `rust/src/core/context_proof.rs`, `rust/src/tools/ctx_proof.rs`, `rust/src/server/dispatch/utility_tools.rs`, `rust/src/tool_defs/granular.rs`, `rust/src/cli/proof_cmd.rs`, `worktrees/deploy-ctxos/website/generated/tool-enrichments.json`

**Evidence (lokal, minimal)**:
- `cd rust && cargo test -q context_proof`
- `cd rust && cargo run -q --bin lean-ctx -- proof --summary --no-write`
- `cd rust && cargo run -q --bin gen_mcp_manifest && cargo test -q --test mcp_manifest_up_to_date`
- `cd worktrees/deploy-ctxos/website && npm run -s generate:tools && npm run -s validate:manifest`

**GitLab Tickets (Context‑OS)**:
- `#2325` Proof Artifact Format v1 → closed

### 2026-05-02 — Replayability Contract v1 + CI Gates (EPIC 4: Verification)

- **4.3 Replayability Contract v1 + CI Gates** (`#2326`):
  - CI gates: `cargo fmt --check`, `cargo clippy -D warnings`, manifest drift gate (`gen_mcp_manifest` + `git diff` + `mcp_manifest_up_to_date`), verification gates (`output_verification`, `context_proof`, `secret_scan_artifacts`)
  - Bench regression subset: `rust/tests/savings_verification.rs` als lightweight threshold gate (allow-failure auf non-default branches; hard-fail auf default/tags)
  - Docs (deploy website): Replayability page dokumentiert jetzt die konkreten Gates + proof export
  - Relevant: `ci/gitlab-ci.yml`, `rust/tests/savings_verification.rs`, `worktrees/deploy-ctxos/website/src/page-templates/DocsReplayabilityPage.astro`

**Evidence (lokal, minimal)**:
- `cd rust && cargo fmt`
- `cd rust && cargo clippy --all-features -- -D warnings`
- `cd rust && cargo run -q --bin gen_mcp_manifest && cargo test -q --test mcp_manifest_up_to_date`
- `cd rust && cargo test -q output_verification && cargo test -q context_proof`
- `cd rust && cargo test -q --test secret_scan_artifacts`
- `cd rust && cargo test -q --test savings_verification`
- `cd worktrees/deploy-ctxos/website && npm run -s validate:doc-claims`

**GitLab Tickets (Context‑OS)**:
- `#2326` Replayability Contract v1 + CI Gates → closed

### 2026-05-02 — Verification Observability v1 (EPIC 4: Verification)

- **4.5 Verification Observability v1** (`#2328`):
  - Tool: `ctx_verify action=stats format=summary|json|both` (versionierter Snapshot; no raw content)
  - CLI: `lean-ctx verify --json`
  - Schema: `VerificationObservabilityV1` (`schema_version=1`) → verifier snapshot + SLO snapshot + budgets + proof counters + pipeline stats
  - Proof counters: `context_proof` tracked collected/written + last_written timestamp (count-only)
  - Deploy website: tool pages + enrichments + docs updated (56 tools)
  - Relevant: `rust/src/core/verification_observability.rs`, `rust/src/tools/ctx_verify.rs`, `rust/src/server/dispatch/utility_tools.rs`, `rust/src/tool_defs/granular.rs`, `rust/src/cli/verify_cmd.rs`

**Evidence (lokal, minimal)**:
- `cd rust && cargo test -q verification_observability`
- `cd rust && cargo run -q --bin lean-ctx -- verify --json`
- `cd rust && cargo run -q --bin gen_mcp_manifest && cargo test -q --test mcp_manifest_up_to_date`
- `cd worktrees/deploy-ctxos/website && npm run -s generate:tools && npm run -s validate:manifest && npm run -s validate:doc-claims`

**GitLab Tickets (Context‑OS)**:
- `#2328` Verification Observability v1 → closed

### 2026-05-02 — Website Deploy Hardening (EPIC 0: Docs/Website)

- **0.3 Website Deploy Hardening** (`#2299`):
  - Deploy build ist jetzt **reproducible**: `website/Dockerfile` ist multi-stage (Node builder → Nginx runtime), kein pre-built dist mehr.
  - Repo hygiene: tracked `website/dist` + tracked `website/node_modules` entfernt; `.gitignore` updated (no un-ignore).
  - CI: `website_build` job nutzt `node:22.12.0`; deploy job asserted `git ls-files website/{dist,node_modules}` == 0.
  - Relevant: `worktrees/deploy-ctxos/.gitlab-ci.yml`, `worktrees/deploy-ctxos/website/Dockerfile`, `worktrees/deploy-ctxos/website/.dockerignore`, `worktrees/deploy-ctxos/.gitignore`

**Evidence (lokal, minimal)**:
- `cd worktrees/deploy-ctxos && git ls-files website/dist | wc -l` → `0`
- `cd worktrees/deploy-ctxos && git ls-files website/node_modules | wc -l` → `0`

**GitLab Tickets (Context‑OS)**:
- `#2299` Website Deploy Hardening → closed

### 2026-05-02 — Proof-first Docs Cross-Linking (EPIC 0: Docs/Website)

- **0.4 Proof-first Docs** (`#2300`):
  - Pillar-Landing-Template verlinkt jetzt Proof pages (mind. Verification + Replayability; Delivery zusätzlich Cookbook) und bleibt locale-aware.
  - Verification docs cross-linken jetzt explizit zu Trust + Replayability + Cookbook (Sidebar) → durchgängige Story ohne dead links.
  - Relevant: `worktrees/deploy-ctxos/website/src/page-templates/DocsContextOsPillarPage.astro`, `worktrees/deploy-ctxos/website/src/page-templates/DocsVerificationPage.astro`

**Evidence (lokal, minimal)**:
- `cd worktrees/deploy-ctxos/website && npm run -s validate:doc-claims`
- `cd worktrees/deploy-ctxos/website && npm run -s validate:tools`

**GitLab Tickets (Context‑OS)**:
- `#2300` Proof-first Docs → closed

### 2026-05-02 — Tools Reference IA v2 (EPIC 0: Docs/Website)

- **0.5 Tools Reference IA v2** (`#2301`) — **closed**:
  - Tools-Reference ist zusätzlich **nach Context‑OS Pillars** navigierbar:
    - `/docs/tools/context-io`
    - `/docs/tools/context-orchestration`
    - `/docs/tools/context-memory`
    - `/docs/tools/context-verification`
    - `/docs/tools/context-delivery`
  - `/docs/tools` zeigt SSOT counts (granular/unified/read-modes) und bietet **globales Search/Filter** über alle Tool-Tabellen.
  - Pillar-Tool-Views haben ebenfalls **Search/Filter** (Name/Description).
  - Tool-Detailseiten zeigen jetzt zusätzlich:
    - **Output contract** (aus `generated/tool-enrichments.json`, wo vorhanden)
    - **Implementation links** (GitHub code paths; enrichment-first, sonst Fallback `rust/src/tools/<tool>.rs` wenn vorhanden)
  - Enrichments erweitert (u.a. `ctx_read`, `ctx_shell`, `ctx_search`, `ctx_tree`, `ctx_proof`, `ctx_verify`) um `output_contract` + `code_paths` + zusätzliche Beispiele.
  - Relevant: `worktrees/deploy-ctxos/website/src/page-templates/DocsToolsPage.astro`, `worktrees/deploy-ctxos/website/src/page-templates/DocsToolsPillarPage.astro`, `worktrees/deploy-ctxos/website/src/page-templates/DocsToolDetailPage.astro`, `worktrees/deploy-ctxos/website/generated/tool-enrichments.json`

**Evidence (lokal, minimal)**:
- `cd worktrees/deploy-ctxos/website && npm run -s validate:doc-claims`
- `cd worktrees/deploy-ctxos/website && npm run -s validate:manifest`
- `cd worktrees/deploy-ctxos/website && npm run -s validate:docs-coverage`
- `cd worktrees/deploy-ctxos/website && npm run -s validate:tools`
- `cd worktrees/deploy-ctxos/website && PATH="/opt/homebrew/opt/node@22/bin:$PATH" npm run -s build`

**GitLab Tickets (Context‑OS)**:
- `#2301` Tools Reference IA v2 → closed

### 2026-05-02 — Docs i18n Hardening (EPIC 0: Docs/Website)

- **0.6 Docs i18n Hardening** (`#2302`) — **closed**:
  - `validate-i18n-duplicates.mjs` ist jetzt ein **Gate**: Build schlägt fehl bei **neuen** duplicate key trees (legacy allowlist bleibt warn-only).
  - Neuer dist-basierter, locale-aware **Linkcheck**: `website/scripts/validate-links.mjs`
    - prüft interne `href="/..."` Links in gebautem `dist/` (inkl. Locale-Routen)
    - unterstützt Astro routing (`/x` → `/x/index.html`) + Sonderfall (`/404` → `/404.html`) + Assets
  - Build pipeline hardened: `npm run build` ruft `validate-links.mjs` nach `astro build` und vor `pagefind` auf.
  - Fix: Broken link `/docs/guides/remote-setup` → `/docs/remote-setup` in `DocsGuideDockerPage.astro`.
  - Relevant: `worktrees/deploy-ctxos/website/scripts/validate-links.mjs`, `worktrees/deploy-ctxos/website/scripts/validate-i18n-duplicates.mjs`, `worktrees/deploy-ctxos/website/package.json`, `worktrees/deploy-ctxos/website/src/page-templates/DocsGuideDockerPage.astro`

**Evidence (lokal, minimal)**:
- `cd worktrees/deploy-ctxos/website && npm run -s validate:i18n`
- `cd worktrees/deploy-ctxos/website && npm run -s validate:i18n-dupes`
- `cd worktrees/deploy-ctxos/website && PATH="/opt/homebrew/opt/node@22/bin:$PATH" npm run -s build`

**GitLab Tickets (Context‑OS)**:
- `#2302` Docs i18n Hardening → closed

### 2026-05-02 — HTTP MCP + Team Server Contracts v1 (EPIC 5: Context Delivery)

- **5.1 HTTP MCP Contract v1** (`#2330`) — **closed**:
  - Host check wired as first gate (unless disabled); loopback defaults preserved.
  - Non-loopback bind requires auth token; `/health` always open.
  - Typed JSON error codes across middleware + `/v1/tools/call`.
  - Integration tests (contract-level): health open, manifest auth, host-deny, rate limit typed error.
  - Docs: new page `/docs/http-mcp` (+ locale route) and contract doc `docs/contracts/http-mcp-contract-v1.md`.
  - Relevant: `rust/src/http_server/mod.rs`, `worktrees/deploy-ctxos/website/src/page-templates/DocsHttpMcpPage.astro`

- **5.2 Team Server Contract v1** (`#2331`) — **closed**:
  - Workspace selection contract: `x-leanctx-workspace` header + `workspaceId` body + deterministic fallback.
  - `rewrite_dot_paths` contract clarified and audited behavior kept deterministic.
  - Audit log hardened: JSONL schema documented; arguments stored as canonicalized `argumentsMd5` only (no raw args).
  - Typed errors for auth/scope/workspace + tool timeout/tool error.
  - Integration tests: scope denied (403) + unknown workspace (400) typed errors.
  - Docs: team server page updated + contract doc `docs/contracts/team-server-contract-v1.md`.
  - Relevant: `rust/src/http_server/team.rs`, `worktrees/deploy-ctxos/website/src/page-templates/DocsTeamServerPage.astro`

**Evidence (lokal, minimal)**:
- `cd rust && cargo test --all-features -q http_contract_v1`
- `cd rust && cargo test --all-features -q scope_denied_is_403_with_typed_error`
- `cd rust && cargo test --all-features -q unknown_workspace_is_400_with_typed_error`
- `cd worktrees/deploy-ctxos/website && PATH="/opt/homebrew/opt/node@22/bin:$PATH" npm run -s build`

**GitLab Tickets (Context‑OS)**:
- `#2330` HTTP MCP Contract v1 → closed
- `#2331` Team Server Contract v1 → closed

### 2026-05-02 — SDK + Cookbook E2E Proof Suite (EPIC 5: Context Delivery)

- **5.3 SDK + Cookbook E2E Proof Suite** (`#2332`) — **closed**:
  - SDK surfaces typed HTTP error codes (`error_code`) via `LeanCtxHttpError.errorCode`.
  - Real E2E test spins up a real `lean-ctx serve` (loopback + auth token) and validates:
    - `/health` is reachable
    - `/v1/manifest` requires auth and returns typed `unauthorized`
    - `/v1/tools` returns stable shape
    - `/v1/tools/call` works via SDK (`ctx_read` + toolText)
  - CI: `cookbook` job now consumes the `lean-ctx` binary artifact from `rust_test` and runs SDK tests against the real server (no mocks).
  - Relevant: `cookbook/sdk/src/client.e2e.test.ts`, `cookbook/sdk/src/client.ts`, `cookbook/sdk/src/errors.ts`, `ci/gitlab-ci.yml`

**Evidence (lokal, minimal)**:
- `cd cookbook && PATH="/opt/homebrew/opt/node@22/bin:$PATH" npm test`

### 2026-05-02 — Premium Integrations v1 (Cursor/Claude Code) (EPIC 5: Context Delivery)

- **5.4 Premium Integrations v1** (`#2333`) — **closed**:
  - Neues Health-Check Kommando: `lean-ctx doctor integrations` (Cursor + Claude Code drift checks: MCP config, hooks, rules).
  - Neuer Repair-Path Alias: `lean-ctx install --repair` → non-interactive `setup --fix` (merge-based, idempotent, no deletes).
  - Cursor `~/.cursor/hooks.json` Installer ist jetzt **merge-based** (preserves other hooks/plugins) statt overwrite.
  - `lean-ctx setup --fix` und `lean-ctx doctor --fix` reparieren jetzt auch Agent Hooks (Cursor/Claude/Codex).
  - `lean-ctx init --agent claude --project` erzeugt `.claude/settings.local.json` (project-local PreToolUse hooks).
  - Docs Hardening: Integrations Guide aktualisiert (repair + health checks, keine placeholder tool lists); Tool-count drift (55) in zentralen Docs/Templates korrigiert; Secret-scan drift fix im HTTP MCP Contract Doc.
  - Relevant: `rust/src/doctor.rs`, `rust/src/cli/dispatch.rs`, `rust/src/cli/init_cmd.rs`, `rust/src/hooks/agents/cursor.rs`, `worktrees/deploy-ctxos/website/src/page-templates/DocsGuideEditorsPage.astro`, `docs/contracts/http-mcp-contract-v1.md`

**Evidence (lokal, minimal)**:
- `cd rust && cargo test --all-features -q cursor_hooks_merge_preserves_other_entries`
- `cd rust && cargo test --all-features -q docs_tool_counts_match_manifest`
- `cd rust && cargo test --all-features -q secret_scan_artifacts`

### 2026-05-02 — Integrations Autotuning (ALL IDEs): Constraints Matrix v1 + CI Gate (Ticket `#2337` / Parent `#2314`)

- **Docs-backed constraints SSOT**:
  - New doc: `docs/integrations/client-constraints-matrix-v1.md` (machine-readable JSON block + cited vendor sources per client).
  - New CI gate: `rust/tests/client_constraints_matrix_up_to_date.rs` ensures the matrix exists, is parseable, is cited (https URLs), and covers all first-class clients.
- **Provider path drift fixes (docs-based)**:
  - Qwen Code uses `~/.qwen/settings.json` (not `~/.qwen/mcp.json`).
  - Amazon Q Developer uses `~/.aws/amazonq/default.json` (legacy `mcp.json` still tolerated for uninstall).
- **Autotuning guardrail**:
  - `autoApprove` is only emitted where vendor docs explicitly document it (currently Cursor + AWS Kiro) to avoid schema drift on other clients.

**Evidence (lokal, minimal)**:
- `cd rust && cargo test --all-features -q client_constraints_matrix_v1_is_complete_and_cited`
- `cd rust && cargo test --all-features -q client_constraints_matrix_matches_runtime_constraints`

### 2026-05-02 — Instruction Compiler v1 (ALL IDEs): deterministic + size-bounded (Ticket `#2338` / Parent `#2314`)

- New runtime SSOT for client constraints: `rust/src/core/client_constraints.rs` (instruction cap + autoApprove support).
- New instruction compiler: `rust/src/core/instruction_compiler.rs`
  - Deterministic output for same inputs (profile + client + flags).
  - Enforces documented caps (Claude Code: 2048 chars for MCP `instructions`).
- New CLI:
  - `lean-ctx instructions --client <id> --profile <name> [--crp off|compact|tdd] [--unified] [--json] [--include-rules]`
  - `lean-ctx instructions --list-clients`
- Drift gate hardening: matrix doc is now checked against runtime caps/autoApprove (`rust/tests/client_constraints_matrix_up_to_date.rs`).

**Evidence (lokal, minimal)**:
- `cd rust && cargo test --all-features -q`
- `cd rust && cargo run -q --bin lean-ctx -- instructions --client claude-code --profile exploration --json`

### 2026-05-02 — Contract Versioning Policy v1 + Compatibility Matrix (EPIC 5: Context Delivery)

- **5.7 Contract Versioning Policy v1** (`#2336`) — **closed**:
  - Root SSOT doc: `CONTRACTS.md` (policy + machine-checked version KV block + compatibility matrix).
  - Runtime versions centralized in `rust/src/core/contracts.rs` and used by schema emitters.
  - CI gate: `rust/tests/contracts_md_up_to_date.rs` prevents drift between runtime versions and `CONTRACTS.md`.
  - Docs: new delivery reference page `/docs/contracts` (root + locale) linking policy + compatibility view.
  - Relevant: `CONTRACTS.md`, `rust/src/core/contracts.rs`, `rust/src/core/mcp_manifest.rs`, `rust/src/core/context_proof.rs`, `rust/src/core/verification_observability.rs`, `worktrees/deploy-ctxos/website/src/page-templates/DocsContractVersioningPage.astro`

**Evidence (lokal, minimal)**:
- `cd rust && cargo test --all-features -q contracts_md_versions_match_runtime`
- `cd worktrees/deploy-ctxos/website && PATH="/opt/homebrew/opt/node@22/bin:$PATH" npm run -s build`

## EPIC P — PR Context Packs (`lean-ctx pack --pr`)

**Status**: Completed  
**Goal**: One command delivers branch-/diff-aware context (changed files, impact, related tests, relevant artifacts, optional hybrid search results).

**GitLab Epic**: `#95`  
**GitLab Subtickets**: `#104`–`#108`  

| Subticket | Description | Status |
|-----------|-------------|--------|
| P1 | Pack format + output contract (`--format markdown\|json`) | Done |
| P2 | Diff/PR input sources (`git diff`, `--base`, `--diff-from-stdin`) | Done |
| P3 | Context assembly pipeline (reuse `ctx_review`, `ctx_impact`, `ctx_graph`) | Done |
| P4 | MCP tool `ctx_pack` + CLI `lean-ctx pack --pr [--base main]` | Done |
| P5 | Demo + proof (tape/GIF) | Done |

**Entrypoints**:
- **CLI**: `lean-ctx pack --pr [--base <ref>] [--json] [--depth <n>] [--diff-from-stdin]`
- **MCP**: `ctx_pack { action:\"pr\", base, format, depth, diff, project_root }`

---

## EPIC A — Codebase Index 1.0

**Status**: Completed  
**Goal**: Reliable index build, update, resume, and status observation — like SocratiCode but local-first.

**GitLab Epic**: `#96`  
**GitLab Subtickets**: `#109`–`#111`  

| Subticket | Description | Status |
|-----------|-------------|--------|
| A1 | BM25 incremental (content hash, no full rebuild, remove 2000-file limit) | Done |
| A2 | Index status contract (public tool) | Done |
| A3 | File watcher + debounce + recovery + cross-process guard | Done |

**Key files**: `rust/src/core/vector_index.rs`, `rust/src/core/index_orchestrator.rs`

**CLI**: `lean-ctx index status|build|build-full|watch` (wired in `rust/src/cli/dispatch.rs` → `rust/src/cli/index_cmd.rs`)  
**MCP**: `ctx_index { action:\"status\"|\"build\"|\"build-full\", project_root }`  

---

## EPIC B — Hybrid Search 2.0

**Status**: Completed  
**Goal**: Single query delivers best results (lexical + semantic), optionally across multiple repos.

**GitLab Epic**: `#97`  
**GitLab Subtickets**: `#112`–`#113`  

| Subticket | Description | Status |
|-----------|-------------|--------|
| B1 | Hybrid search as first-class workflow (`hybrid\|dense\|bm25` modes, tree-sitter chunk boundaries) | Done |
| B2 | Linked projects / workspace search (`.leanctx.json` `linkedProjects`, cross-repo RRF fusion) | Done |

**Key files**: `rust/src/core/hybrid_search.rs`, `rust/src/tools/ctx_semantic_search.rs`

---

## EPIC C — Context Artifacts

**Status**: Completed  
**Goal**: Non-code knowledge (schemas, OpenAPI, infra, architecture docs) is searchable and packable just like code.

**GitLab Epic**: `#98`  
**GitLab Subtickets**: `#114`–`#116`  

| Subticket | Description | Status |
|-----------|-------------|--------|
| C1 | Artifact registry (`.leanctxcontextartifacts.json`) | Done |
| C2 | Artifact index + staleness detection + incremental update | Done |
| C3 | Artifact tools (`ctx_artifacts` actions + integration into search/pack) | Done |

**Key files**: `rust/src/core/context_artifacts.rs`, `rust/src/core/artifact_index.rs`

**Entrypoints**:
- **MCP**: `ctx_artifacts` actions `list|status|index|reindex|search|remove`
- **Integration**: `ctx_pack` includes relevant artifacts; `ctx_semantic_search` supports workspace and artifact-related flows

---

## EPIC D — Stable Project IDs + Branch-Aware

**Status**: Completed  
**Goal**: Indexes are worktree-/clone-stable; optionally branch-aware for CI/PR.

**GitLab Epic**: `#99`  
**GitLab Subtickets**: `#117`–`#118`  

| Subticket | Description | Status |
|-----------|-------------|--------|
| D1 | `project_identity` as index key + auto-migration from legacy path-hash dirs | Done |
| D2 | Branch-aware switch (`LEANCTX_INDEX_BRANCH_AWARE=true`) | Done |

**Key files**: `rust/src/core/project_hash.rs`

---

## EPIC E — Optional Qdrant Backend

**Status**: Completed  
**Goal**: Scaling & team server can use Qdrant without breaking default single-binary DNA.

**GitLab Epic**: `#100`  
**GitLab Subtickets**: `#119`–`#120`  

| Subticket | Description | Status |
|-----------|-------------|--------|
| E1 | Storage abstraction (`core::dense_backend`: local vs qdrant behind feature flag) | Done |
| E2 | Config + telemetry safety (URL, API key, namespace hashing, audit records) | Done |

**Key files**: `rust/src/core/qdrant_store.rs`, `rust/src/core/dense_backend.rs`

---

## EPIC F — Self-Hosted Team Server

**Status**: Completed  
**Commit**: `1a802efcd`  
**Goal**: Shared index + artifacts + graph without managed cloud.

**GitLab Epic**: `#101`  
**GitLab Subtickets**: `#121`–`#123`  

| Subticket | Description | Status |
|-----------|-------------|--------|
| F1 | Server mode architecture (reuse `lean-ctx serve` Streamable HTTP MCP) | Done |
| F2 | AuthN/AuthZ (scoped API tokens: search/graph/artifacts/index, audit log JSONL) | Done |
| F3 | Multi-repo org setup (workspace management, `lean-ctx team sync`) | Done |

**Key files**: `rust/src/http_server/team.rs`, `rust/src/cli/dispatch.rs`  
**CLI**: `lean-ctx team serve`, `lean-ctx team token create`, `lean-ctx team sync`

---

## EPIC G — Interactive Graph Explorer

**Status**: Completed  
**Commit**: `1a802efcd`  
**Goal**: "Show me the graph" is one command, shareable.

**GitLab Epic**: `#102`  
**GitLab Subticket**: `#124`  

| Subticket | Description | Status |
|-----------|-------------|--------|
| G1 | Self-contained HTML export with pan/zoom, node selection, transitive highlighting, PNG export | Done |

**Key files**: `rust/src/core/graph_export.rs`  
**CLI**: `lean-ctx graph export-html`  
**MCP**: `ctx_graph action=export-html`

---

## EPIC H — Verification: Benchmarks, Tests, Security

**Status**: Completed  
**Commits**: `1a802efcd`, `26be098d6`

**GitLab Epic**: `#103`  
**GitLab Subtickets**: `#125`–`#127`  

| Subticket | Description | Status |
|-----------|-------------|--------|
| H1 | Real-world benchmark suite (grep vs `ctx_semantic_search` + `ctx_graph/impact`) | Done |
| H2 | Regression tests (incremental index, watcher, linkedProjects RRF, artifact search) | Done |
| H3 | Security hardening (pathjail for artifacts/linkedProjects, rate limits in team server) | Done |

**Key files**: `rust/src/core/workspace_config.rs` (pathjail integration)

---

## Context OS — Phase 2 (Infrastruktur-Standardisierung) — Execution Log

### 2026-05-02 — Context IR v1 + Pipeline Unification (metrics-only first)

**Ziel**: Ein einheitliches, versioniertes IR für alle I/O Quellen (read/shell/search/provider), plus per-layer Metrics Export — ohne Big-Bang Refactor.

- **ContextIrV1 (bounded + redaction-safe)**:
  - Neues Schema + persistenter Store: `rust/src/core/context_ir.rs`
  - Contract SSOT: `rust/src/core/contracts.rs` + `CONTRACTS.md`
  - Contract doc: `docs/contracts/context-ir-v1.md`
- **Inkrementelle Collection (kein Behavior-Drift)**:
  - `ctx_read`: IR + Pipeline-Metrics (Compression-layer) beim Read-Dispatch
  - `ctx_shell` + `ctx_search`: IR + Pipeline-Metrics (Compression-layer) beim Dispatch
- **Proof Export referenziert IR**:
  - `lean-ctx proof` exportiert zusätzlich `context-ir-v1_<timestamp>.json` nach `project/.lean-ctx/proofs/`

**Evidence (lokal)**:
- `cd rust && cargo test --all-features`
- `cd rust && cargo clippy --all-features -- -D warnings`

**GitLab Tickets (Context‑OS)**:
- `#2308` Context IR v1 + Pipeline Unification → closed

### 2026-05-02 — Intent→Mode→Budget Router v1 (Policy Contract)

**Ziel**: Routing als testbarer Policy Contract (statt nur Heuristik): Intent → Model Tier Empfehlung + Read-Mode Empfehlung + deterministische Degradation unter Budget/Pressure.

- **IntentRouteV1 Contract + Policy**:
  - Router Schema + deterministische Reasoning-Fields: `rust/src/core/intent_router.rs`
  - Profile Overrides: `[routing]` in `rust/src/core/profiles.rs` (z.B. `hotfix` capped auf `fast`)
  - Contract SSOT: `rust/src/core/contracts.rs` + `CONTRACTS.md`
  - Contract doc: `docs/contracts/intent-route-v1.md`
- **Runtime surface**:
  - `ctx_intent` unterstützt `format=json` und liefert `IntentRouteV1` als JSON (ansonsten compact ack)
  - Tool schema aktualisiert: `rust/src/tool_defs/granular.rs`
  - SSOT manifest regeneriert: `rust/bin/gen_mcp_manifest` → `website/generated/mcp-tools.json`

**Evidence (lokal)**:
- `cd rust && cargo test --all-features`
- `cd rust && cargo clippy --all-features -- -D warnings`
- `cd rust && cargo test --test mcp_manifest_up_to_date`
- `cd rust && cargo test --test contracts_md_up_to_date`

**GitLab Tickets (Context‑OS)**:
- `#2309` Intent→Mode→Budget Router v1 → closed

### 2026-05-02 — Budget/SLO Degradation Policy v1 (Warn/Throttle/Block, policy-backed)

**Ziel**: Degradation als versionierter Policy Contract, deterministisch und überall gleich (MCP/HTTP/Team). Default bleibt **warn-only**; Enforcement ist explizit hinter Profile-Config.

- **DegradationPolicyV1 Contract**:
  - Contract + ladder + reason_code: `rust/src/core/degradation_policy.rs`
  - Contract SSOT: `rust/src/core/contracts.rs` + `CONTRACTS.md`
  - Contract doc: `docs/contracts/degradation-policy-v1.md`
- **Enforcement (ein Boundary für alle Surfaces)**:
  - `rust/src/server/mod.rs` (`call_tool`): wendet Policy konsistent an
    - Budget-based Block bleibt role-gated (`block_at_percent < 255`)
    - SLO Throttle/Block wird nur enforced wenn Profile `[degradation].enforce=true`
- **Proof Export**:
  - `lean-ctx proof` schreibt zusätzlich `degradation-policy-v1_<timestamp>.json` nach `project/.lean-ctx/proofs/` (`rust/src/tools/ctx_proof.rs`)

**Evidence (lokal)**:
- `cd rust && cargo test --all-features`
- `cd rust && cargo clippy --all-features -- -D warnings`
- `cd rust && cargo test --test contracts_md_up_to_date`

**GitLab Tickets (Context‑OS)**:
- `#2312` Budget/SLO Degradation Policy v1 → closed

### 2026-05-02 — Workflow Evidence Ledger v1 (tool receipts + proof artifacts + evidence-gated transitions)

**Ziel**: Evidence Inputs standardisieren und auditierbar machen: Tool Receipts + Proof Artefakte + manual Evidence als bounded, content-addressed Ledger; Workflows können Transitions zuverlässig über Evidence Keys gaten.

- **WorkflowEvidenceLedgerV1 Contract**:
  - Contract SSOT: `rust/src/core/contracts.rs` + `CONTRACTS.md`
  - Contract doc: `docs/contracts/workflow-evidence-ledger-v1.md`
  - Runtime: `rust/src/core/evidence_ledger.rs` (bounded store, redaction-safe excerpts, deterministic IDs)
- **Automatic evidence capture**:
  - Tool-call boundary schreibt Tool Receipts in Ledger: `rust/src/server/mod.rs`
  - `ctx_workflow evidence_add` schreibt manual Evidence in Ledger: `rust/src/tools/ctx_workflow.rs`
  - `lean-ctx proof` schreibt Proof Artefakte als Evidence (`proof:*` Keys): `rust/src/tools/ctx_proof.rs`

**Evidence (lokal)**:
- `cd rust && cargo test --all-features`
- `cd rust && cargo clippy --all-features -- -D warnings`
- `cd rust && cargo test --test contracts_md_up_to_date`
- `cd rust && cargo test --test mcp_manifest_up_to_date`

**GitLab Tickets (Context‑OS)**:
- `#2315` Workflow Evidence Ledger v1 → closed

### 2026-05-02 — Autonomy Drivers v1 (preload/prefetch/dedup/response) als state machine + proofs

**Ziel**: Deterministische Helper‑Driver (kein “full autonomy”), die guarded (Budget/SLO/Boundary) laufen und als Proof/Report exportierbar sind: **welche Driver liefen + warum**.

- **AutonomyDriversV1 Contract + Store (bounded, redaction-safe)**:
  - Contract SSOT: `rust/src/core/contracts.rs` + `CONTRACTS.md`
  - Contract doc: `docs/contracts/autonomy-drivers-v1.md`
  - Runtime store: `rust/src/core/autonomy_drivers.rs` (`~/.lean-ctx/autonomy_drivers_v1.json`)
- **Deterministischer Driver Planner + Guards**:
  - Profile-gated (opt-in): `rust/src/core/profiles.rs` (`[autonomy]`, neues built-in Profil `coder`)
  - Budget/SLO guard via `DegradationPolicyV1` (Throttle/Block → skip)
  - Boundary: alle file reads laufen über `io_boundary::jail_and_check_path` (skip secret-like paths)
- **Wiring (Pipeline + Output Metadata + Proofs)**:
  - Session start: auto preload/overview pre-hook (`rust/src/tools/autonomy.rs`), emits bounded driver report
  - After read: optional bounded prefetch + dedup (opt-in) (`rust/src/tools/autonomy.rs`)
  - Post call: optional response shaping für große Outputs (`rust/src/tools/autonomy.rs` + `rust/src/server/mod.rs`), **nie** für JSON outputs
  - Pipeline: neue Layer-Kind `autonomy` + metrics record bei response shaping (`rust/src/core/pipeline.rs`, `rust/src/server/mod.rs`)
  - Proof export: `lean-ctx proof` schreibt zusätzlich `autonomy-drivers-v1_<timestamp>.json` nach `project/.lean-ctx/proofs/` (`rust/src/tools/ctx_proof.rs`)
  - Evidence: Proof-Artefakt wird als `proof:autonomy-drivers-v1` ins Evidence Ledger recorded
- **Security Fixes**:
  - `ctx_preload` + `ctx_prefetch` jailed reads (kein Path traversal / kein secret-like preload)

**Evidence (lokal)**:
- `cd rust && cargo fmt`
- `cd rust && cargo test --all-features -q`
- `cd rust && cargo clippy --all-features -- -D warnings`
- `cd rust && cargo test -q --test contracts_md_up_to_date`
- `cd rust && cargo test -q --test mcp_manifest_up_to_date`

**GitLab Tickets (Context‑OS)**:
- `#2313` Autonomy Drivers v1 → closed

### 2026-05-03 — Tokenizer-aware Translation Driver v1 (TokenOptimizer calibrated per model family)

**Ziel**: Translation wird tokenizer-aware und modell-/profile-gesteuert, ohne Default-Format-Breaking-Changes: Unicode→ASCII nur bei opt-in + deterministischer Auswahl.

- **TokenizerTranslationDriverV1 Contract + SSOT**:
  - Contract SSOT: `rust/src/core/contracts.rs` + `CONTRACTS.md`
  - Contract doc: `docs/contracts/tokenizer-translation-driver-v1.md`
  - Runtime: `rust/src/core/tokenizer_translation_driver.rs`
- **Policy (safe defaults, opt-in)**:
  - Neues Profile-Segment: `translation.enabled` + `translation.ruleset = legacy|ascii|auto` (`rust/src/core/profiles.rs`)
  - Built-in Profil `coder` nutzt `translation.enabled=true` + `ruleset=auto`
- **Deterministische Ruleset Selection**:
  - `LEAN_CTX_MODEL` / `LCTX_MODEL` → model_key → ruleset (`auto`: OpenAI/GPT → ASCII, sonst legacy)
  - JSON Outputs werden **nie** übersetzt (machine-readable bleibt exakt)
- **Wiring + Verifier Safety**:
  - Tool-call boundary wendet Translation an (nur wenn enabled) + Pipeline Metrics (`LayerKind::Translation`): `rust/src/server/mod.rs`
  - Verifier bleibt Gate: Path/Identifier checks laufen auf “vorher vs nachher” (`rust/src/core/output_verification.rs`)
- **Bench / Measurement**:
  - `ctx_benchmark` zeigt zusätzlich `signatures (tdd, ascii)` token_cost als Vergleich (`rust/src/tools/ctx_benchmark.rs`)
- **Determinism Fix**:
  - `TokenOptimizer` rule application order ist jetzt deterministisch (Vec+sort statt HashMap iteration): `rust/src/core/neural/token_optimizer.rs`

**Evidence (lokal)**:
- `cd rust && cargo fmt`
- `cd rust && cargo test --all-features -q`
- `cd rust && cargo clippy --all-features -- -D warnings`
- `cd rust && cargo test -q --test contracts_md_up_to_date`
- `cd rust && cargo test -q --test mcp_manifest_up_to_date`

**GitLab Tickets (Context‑OS)**:
- `#2310` Tokenizer-aware Translation Driver v1 → closed

### 2026-05-03 — Attention-aware Layout Driver v1 (learned L-curve + context_reorder integration)

**Ziel**: Delivery nutzt attention-aware ordering (semantic chunks first, line-level fallback) als **opt-in** pro Profile; deterministisch (input+keywords) und verifier-safe (nur Umsortierung, kein Drop).

- **AttentionLayoutDriverV1 Contract + SSOT**:
  - Contract SSOT: `rust/src/core/contracts.rs` + `CONTRACTS.md`
  - Contract doc: `docs/contracts/attention-layout-driver-v1.md`
  - Runtime: `rust/src/core/attention_layout_driver.rs`
- **Policy (opt-in)**:
  - Neues Profile-Segment: `layout.enabled` + `layout.min_lines` (`rust/src/core/profiles.rs`)
  - Built-in Profil `review`: `layout.enabled=true` (Default `exploration` bleibt unverändert/off)
- **Determinismus (tie-breaks)**:
  - Line-level reorder: stable sort tie-break via `original_index` (`rust/src/core/neural/context_reorder.rs`)
  - Chunk reorder: stable sort tie-break via `start_line` (`rust/src/core/semantic_chunks.rs`)
- **Wiring (Delivery surface)**:
  - `ctx_read` (full) wendet reorder vor SymbolMap/Header an, **nur** wenn `task` Keywords vorhanden (`rust/src/tools/ctx_read.rs`)
  - Keywords via `task_relevance::parse_task_hints` (task/intent)

**Evidence (lokal)**:
- `cd rust && cargo fmt`
- `cd rust && cargo test --all-features -q`
- `cd rust && cargo clippy --all-features -- -D warnings`
- `cd rust && cargo test -q --test contracts_md_up_to_date`
- `cd rust && cargo test -q --test mcp_manifest_up_to_date`

**GitLab Tickets (Context‑OS)**:
- `#2311` Attention-aware Layout Driver v1 → closed

### 2026-05-03 — CCP Session Bundle v1 (Session Export/Import Contract, redacted-by-default)

**Ziel**: Standardisiertes, replaybares Export/Import-Format für Session-Ausschnitte (Task/Findings/Decisions/Evidence/State) mit **deterministischen IDs**, **Boundedness** (MAX bytes) und **Default-Redaction**.

- **CcpSessionBundleV1 Contract + SSOT**:
  - Contract SSOT: `rust/src/core/contracts.rs` + `CONTRACTS.md`
  - Contract doc: `docs/contracts/ccp-session-bundle-v1.md`
  - Runtime: `rust/src/core/ccp_session_bundle.rs`
- **Tool Surface (`ctx_session`)**:
  - Neue Actions: `export`, `import` (`rust/src/tools/ctx_session.rs`)
  - Dispatcher Options: `format`, `path`, `write`, `privacy` (`rust/src/server/dispatch/session_tools.rs`)
  - Tool schema erweitert (Args + enums): `rust/src/tool_defs/granular.rs`
- **Security/Privacy + Boundedness**:
  - Export/Import via `io_boundary::jail_and_check_path` (kein Path traversal / keine secret-like locations)
  - Default `privacy=redacted`; `privacy=full` nur für `admin` Role
  - Size Cap: `MAX_BUNDLE_BYTES` enforced bei serialize/read
  - Import markiert `files_touched[*].stale=true` wenn Pfade fehlen/außerhalb Jail; warnt bei `project_identity_hash` mismatch
- **Compatibility (Session State)**:
  - `FileTouched.stale` mit `#[serde(default)]` → alte Sessions bleiben loadbar (`rust/src/core/session.rs`)

**Evidence (lokal)**:
- `cd rust && cargo fmt`
- `cd rust && cargo test --all-features`
- `cd rust && cargo clippy --all-features -- -D warnings`

**GitLab Tickets (Context‑OS)**:
- `#2316` CCP Session Bundle v1 → closed

### 2026-05-03 — Knowledge Policy Contract v1 (bounded governance, budgets, profile overrides)

**Ziel**: Versionierter Policy-Vertrag für Knowledge (Facts/Patterns/History + Relations), der **bounded**, **deterministisch**, **replayable** und **auditable** ist.

- **KnowledgePolicy Contract + SSOT**:
  - Contract SSOT: `rust/src/core/contracts.rs` + `CONTRACTS.md`
  - Contract doc: `docs/contracts/knowledge-policy-contract-v1.md`
  - Runtime: `rust/src/core/memory_policy.rs` + `rust/src/core/knowledge.rs`
- **Policy Surface (Config + Profile Overrides)**:
  - `KnowledgePolicy` Budgets: `recall_facts_limit`, `rooms_limit`, `timeline_limit`, `relations_limit`
  - Profile Overrides: `Profile.memory` (Option-Overrides) + deterministic merge (`rust/src/core/profiles.rs`)
  - Effective policy load+validate (inkl. overrides) in Tools (`rust/src/tools/ctx_knowledge.rs`, `rust/src/tools/ctx_knowledge_relations.rs`)
- **Semantik (stabil, deterministisch)**:
  - Contradiction severity: High/Medium/Low (threshold + confirmations)
  - Similarity guard verhindert false contradictions bei semantisch gleichen Werten
  - Supersedes chain: archived→current via deterministische `fact_version_id_v1` (MD5)
- **Tool Surface**:
  - `ctx_knowledge action=policy value=show|validate` (effective policy + range validation)
  - Budgets enforced für `recall`/`rooms`/`timeline`/`relations` (stable ordering + truncation)

**Evidence (lokal)**:
- `cd rust && cargo fmt`
- `cd rust && cargo test --all-features -q`
- `cd rust && cargo clippy --all-features -- -D warnings`
- `cd rust && cargo test -q --test contracts_md_up_to_date`
- `cd rust && cargo test -q --test mcp_manifest_up_to_date`

**GitLab Tickets (Context‑OS)**:
- `#2317` Knowledge Policy Contract v1 → closed

### 2026-05-03 — Graph Reproducibility Contract v1 (format=json + freshness + architecture proof artifacts)

**Ziel**: Graph-Tools (Property Graph) werden **reproducible** (CI/Proofs), **deterministisch** (stable ordering), **bounded** (Output caps) und liefern maschinenlesbare Exporte.

- **Graph Reproducibility Contract + SSOT**:
  - Contract SSOT: `rust/src/core/contracts.rs` + `CONTRACTS.md`
  - Contract doc: `docs/contracts/graph-reproducibility-contract-v1.md`
- **Runtime (Determinismus + Freshness)**:
  - Property graph meta: `.lean-ctx/graph.meta.json` (`rust/src/core/property_graph/meta.rs`)
  - Deterministische Adjacency + stable BFS/DFS: `rust/src/core/property_graph/queries.rs`
  - Deterministischer Build (sorted file enumeration + sorted import targets): `rust/src/tools/ctx_impact.rs`
  - Deterministische Architecture Outputs + bounded previews: `rust/src/tools/ctx_architecture.rs`
- **Tool Surface (`format=json`)**:
  - `ctx_impact`: `analyze|chain|build|status` + `format=text|json` (inkl. `project_identity_hash`, `graph_meta`, truncation markers)
  - `ctx_architecture`: `overview|clusters|layers|cycles|entrypoints|module` + `format=text|json`
  - Tool schemas: `rust/src/tool_defs/granular.rs` + Manifest SSOT regen (`cargo run --bin gen_mcp_manifest`)
- **Proof Artifacts (Architecture Overview)**:
  - `ctx_proof write=true` schreibt zusätzlich:
    - `.lean-ctx/proofs/architecture-overview-v1_<ts>.json`
    - `.lean-ctx/proofs/architecture-overview-v1_<ts>.html`
  - EvidenceLedger keys: `proof:architecture-overview-v1` + `proof:architecture-overview-v1-html`

**Evidence (lokal)**:
- `cd rust && cargo fmt`
- `cd rust && cargo test --all-features -q`
- `cd rust && cargo clippy --all-features -- -D warnings`
- `cd rust && cargo test -q --test contracts_md_up_to_date`
- `cd rust && cargo test -q --test mcp_manifest_up_to_date`
- `cd rust && cargo test -q --all-features --test property_graph_reproducibility_contract`

**GitLab Tickets (Context‑OS)**:
- `#2318` Graph Reproducibility Contract v1 → closed

### 2026-05-03 — A2A Contract v1 (privacy/TTL + rate limiting + cost attribution + export snapshot)

**Ziel**: A2A primitives (Messages/Tasks/Diaries) werden deterministisch, bounded und privacy-safe; zusätzlich gibt es ein maschinenlesbares Snapshot‑Artefakt für Proofs/Audits.

- **Contract (SSOT)**:
  - `CONTRACTS.md` + runtime versions: `rust/src/core/contracts.rs`
  - Contract doc: `docs/contracts/a2a-contract-v1.md`
- **Messages (Privacy + TTL + Project Scope)**:
  - A2A Semantik: `rust/src/core/a2a/message.rs`
  - Persisted scratchpad: `rust/src/core/agents.rs` (`ScratchpadEntry` erweitert: `privacy`, `priority`, `expires_at`, `project_root`, `metadata`)
  - Enforcement:
    - `privacy=private` erfordert `to_agent`
    - `ttl_hours>=1` ⇒ deterministic expiry cleanup
    - project-scoped visibility (nur matching `project_root`)
- **Tool Surface (`ctx_agent`, `ctx_task`)**:
  - `ctx_agent` erweitert: `privacy`, `priority`, `ttl_hours`, `format`, `write`, `filename`, `action=export` (`rust/src/tools/ctx_agent.rs`)
  - Dispatcher erweitert: `rust/src/server/dispatch/session_tools.rs`
  - `ctx_task` state machine: `rust/src/core/a2a/task.rs` + `rust/src/tools/ctx_task.rs`
- **Rate limiting (fairness)**:
  - token bucket limiter: `rust/src/core/a2a/rate_limiter.rs` (env overrides; `retry_after_ms`)
  - enforced at MCP tool boundary: `rust/src/server/dispatch/mod.rs`
- **Cost attribution (inkl. cached tokens)**:
  - Store + pricing integration: `rust/src/core/a2a/cost_attribution.rs`
  - Reporting: `rust/src/tools/ctx_cost.rs` + `ctx_gain action=cost`
  - Tool-call boundary passes `cached_tokens` when provided: `rust/src/server/mod.rs`
- **Proof artifact (A2A Snapshot v1)**:
  - `ctx_agent action=export write=true` schreibt `.lean-ctx/proofs/a2a-snapshot-v1_<ts>.json`
  - EvidenceLedger key: `proof:a2a-snapshot-v1`

**Evidence (lokal)**:
- `cd rust && cargo fmt`
- `cd rust && cargo test --all-features -q`
- `cd rust && cargo clippy --all-features -- -D warnings`
- `cd rust && cargo test -q --test contracts_md_up_to_date`
- `cd rust && cargo test -q --test mcp_manifest_up_to_date`

**GitLab Tickets (Context‑OS)**:
- `#2319` A2A Contract v1 → closed

---

## Additional Fixes (v3.4.7 Release Cycle)

### GitHub Issues

| Issue | Title | Fix | Commit |
|-------|-------|-----|--------|
| #173 | Pi MCP bridge sends `paths` as JSON-encoded string | `get_str_array` handles both native arrays and JSON-encoded strings | `1a802efcd` |
| #174 | `ctx_discover_tools` not invocable with static-registry clients | New `ctx_call` meta-tool in `CORE_TOOL_NAMES` | `1a802efcd` |

### Discord Bug Reports

| Bug | Reporter | Root Cause | Fix | Commit |
|-----|----------|------------|-----|--------|
| Pipe guard false positive on Windows Git Bash | knindza94 | `rc_has_pipe_guard` only checked legacy paths, not XDG; backslashes not handled | `doctor.rs` + `shell_init.rs` refactored with stable begin/end markers, bash-compatible path conversion | `46f3996ce` |
| Claude Code hooks not intercepting | knindza94 | `extract_json_field` failed on spaced JSON; hook install overwrote other plugins | Robust JSON parsing + merge-based `ensure_command_hook` | `b432518ab`, `4672fce86` |

### IDE Integration Audit

| Change | Files | Commit |
|--------|-------|--------|
| Dual-format hook output (Cursor + Claude Code compatible) | `hook_handlers.rs` | `31dbf8229` |
| JetBrains `mcpServers` snippet format | `writers.rs`, `jetbrains.rs` | `86d329924` |

**Verified IDEs**: Cursor, VS Code, Claude Code, JetBrains, Codex, OpenCode, Gemini CLI

---

## Release v3.4.7

**Tag**: `v3.4.7` | **Date**: 2026-05-01 | **Workflow**: `#25227853872` (11/11 green)

| Channel | Version | Status |
|---------|---------|--------|
| GitHub Release | v3.4.7 (9 assets) | Published |
| crates.io | 3.4.7 | Published |
| npm lean-ctx-bin | 3.4.7 | Published |
| npm pi-lean-ctx | 3.4.7 | Published |
| Homebrew | 3.4.7 (auto-updated) | Published |
| AUR lean-ctx | 3.4.7 | Pushed |
| AUR lean-ctx-bin | 3.4.7 | Pushed |
| leanctx.com | version.txt → 3.4.7 | Deployed |
| GitLab mirror | main synced | Pushed |
