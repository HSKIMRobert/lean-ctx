# Project Brief — LeanCTX Token-Control-Platform

## Zweck

LeanCTX ist eine local-first, customer-owned Runtime zur Verarbeitung und
Kontrolle von AI-Tokenströmen. Sie verbindet Context Engineering, Gateway,
Policy Enforcement, Routing, Memory, Agent-to-Agent-Kontrolle und
wirtschaftliche Evidenz in einer Plattform.

## North Star

> Every enterprise AI token is observable, controllable and attributable —
> inside the customer's trust boundary.

Der vollständige adressierbare Tokenstrom eines Unternehmens soll über LeanCTX
laufen können. Thinkery benötigt keine zentralen Kundendaten.

## Drei Planes

- **Token Data Plane:** Gateway, MCP, SDK, Sidecar und Shell verarbeiten Traffic.
- **Token Control Plane:** Policies, Budgets, Modelle, Identitäten und Experimente.
- **Token Value & Evidence Plane:** Ledger, Outcomes, Savings, Audit und Abrechnung.

## Architektur

- 5 Produktschichten: Engine, OCLA, Ledger, Interception Points, AI Value Gate.
- 14 OCLA Capabilities in 4 Token-Kontrolldimensionen.
- Rust Traits plus versionierter Wire Contract.
- providerneutrales Canonical Token Envelope für Request, Stream, Tool, Usage und Error.
- Strangler-Fig-Migration ohne Big-Bang-Rewrite.
- Gesamtprogramm W0–W10 mit blockierenden Gates G0–G10; P0–P11 sind OCLA Work-Packages.

## Geschäftsmodell

Thinkery monetarisiert:

1. Setup und Integration;
2. Enterprise Subscription;
3. Support und Betrieb;
4. gedeckelten Anteil verifizierter Netto-Savings.

Open Source schafft Distribution, Prüfbarkeit und Integrationsstandard. Das
kommerzielle Angebot verkauft Enterprise-Skalierung, Assurance und messbaren
Wert, nicht den Zugriff auf Kundendaten.

## Nicht verhandelbar

- local-first und customer-owned;
- Zero Telemetry by default;
- Data Plane unabhängig von Cloud und Gate;
- deterministische und versionierte Contracts;
- keine doppelte Savings-Attribution;
- keine lokale Kernfunktion hinter Paywall;
- Shadow Mode vor Enforcement;
- Safety, Rollback und Human Approval für ACT.
- Semantic Fidelity und explizite Capability Gaps; kein stiller Protokollverlust.
- kein GA-Claim ohne externe Integration und zweites fork-freies Deployment.

## Source of Truth

- `docs/business/gateway-integration/token-control-platform.md`
- `docs/business/product-architecture.md`
- `docs/business/gateway-integration/premium-transformation-program.md`
- `docs/business/gateway-integration/master-plan.md`
- `docs/business/gateway-integration/spec.md`
- `docs/business/gateway-integration/requirements-traceability.md`
- `docs/business/gateway-integration/decisions.md`
