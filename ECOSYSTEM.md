# LeanCTX — Ecosystem Overview

> Product vision: [`VISION.md`](VISION.md)

Software ate the world. Agents are eating software. And every agent is exactly
as good as the context it is given — context decides what an agent knows, what
it may do, and what it provably did. Today that context is unmanaged: untyped
markdown, copy-pasted prompts, vendor-locked memory, zero provenance.

**LeanCTX makes context infrastructure**: efficient, verifiable, portable,
organizational — managed with the same rigor as code.

## One product, three pillars

| Pillar | What it does | Who it serves |
|--------|-------------|---------------|
| **Engine** | Active token reduction — compression, MCP tools, agent hooks, local dashboard | Individual developers |
| **Gateway** | Org-wide LLM proxy — usage tracking, budget enforcement, FinOps, compliance | Companies, Ops, FinOps, CISOs |
| **Cloud** | Hosted services — accounts, sync, team provisioning, billing, registry | Teams, Pro users, managed enterprise |

The **Engine** ships in the open-source binary (`lean-ctx`). **Gateway** and **Cloud** capabilities are available through `lean-ctx-enterprise` (see [ADR-023](docs/business/adr-023-open-core-split.md)). Enterprise tiers add org-scale governance, hosting, and managed infrastructure.

### Engine

The developer-facing context compression layer: 10 read modes, 95+ shell
compression patterns, tree-sitter AST for 27 languages, semantic search with
hybrid BM25 + dense retrieval, session continuity protocol (~400 tokens instead
of ~50K cold start). Ships as MCP tools (`ctx_read`, `ctx_compose`,
`ctx_search`, …) and transparent agent hooks.

### Gateway

The org-wide LLM reverse proxy: intercepts Anthropic, OpenAI, Gemini and
ChatGPT traffic, compresses prompts in-flight, meters per-request cost
attribution, enforces budgets, and provides FinOps dashboards. Available through `lean-ctx-enterprise` ([ADR-023](docs/business/adr-023-open-core-split.md)).

### Cloud

The hosted coordination layer: user accounts, team provisioning, knowledge
sync, billing edge, and the context package registry. Enables Pro/Team/
Enterprise tiers without requiring self-hosted infrastructure.

## Companion projects

| Project | Role |
|---------|------|
| **ctxpkg.org** | Open standard for signed, typed context packages (`.ctxpkg`) |
| **ctxpkg.com** | Registry and marketplace for context packages |

## Doctrines

- **Zero telemetry, absolutely** — nothing leaves a machine automatically;
  explicit, locally computed, user-invoked shares only.
- **Trust is never for sale** — no paid placement, ranking or verification on
  any surface.
- **Local-Free Invariant** — every feature works self-hosted for free;
  commercial tiers add hosting and support, never capabilities.
- **Distilled, typed, signed knowledge only** — never raw transcripts.
