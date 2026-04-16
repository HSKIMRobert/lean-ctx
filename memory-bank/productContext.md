# Product Context

## Warum das existiert
LLM-Agenten scheitern nicht primär an „Intelligenz", sondern an **fehlender Process Authority** und **schlechtem Context Flow**:
- Sie lesen zu viel/zu wenig.
- Sie benutzen Tools inkonsistent.
- Sie liefern „Done"-Behauptungen ohne Evidence.

`lean-ctx` macht Context + Prozess **deterministisch**, **lokal**, **messbar**.

## Nutzererlebnis (Soll)
- Agent arbeitet **schneller** (weniger Tokens, weniger IO) und **zuverlässiger** (Rails + Evidence).
- Nutzer kann `lean-ctx` wie heute nutzen (Shell Hook + MCP stdio), oder als Runtime in Harnesses/Orchestratoren einbetten.
- Website/Docs zeigen **niemals** falsche Zahlen (SSOT/Manifest + i18n Gate).

## Haupt-User-Stories
- Als Agent: „Gib mir nur das Nötige" → `ctx_read` mit `auto/task/reference/lines` + Cache.
- Als Developer: „Zeig Impact" → `ctx_impact`, `ctx_architecture`, `ctx_graph`.
- Als Orchestrator: „Zwinge Process" → `ctx_workflow` + Gatekeeper + Evidence Receipts.
- Als Team (local-first): „Was kostet uns Tooling?" → `ctx_cost` report, ohne Cloud.

## Zwei Betriebsmodi
1. **MCP Server** (primary) — Editoren rufen lean-ctx Tools via Model Context Protocol
2. **Shell Hook** (secondary) — transparente Command-Kompression via Aliases

## Vision
Context Runtime for AI Agents — nicht nur Token-Kompression, sondern vollständige Context-Orchestrierung, Process Rails, und Knowledge Management.
