# Architecture Decision Log

## ADR-001: bounded_lock statt raw blocking_read/write (2026-05-20)

**Kontext**: WSL2/NFS User (ReDev1L) erlebten random Freezes bei ctx_read. `tokio::sync::RwLock::blocking_read()` hat kein Timeout.

**Entscheidung**: Neues `server/bounded_lock.rs` Modul mit `read()`/`write()` Wrappern die `io_health::adaptive_timeout()` nutzen. Return `Option<Guard>` — None bei Timeout für graceful Fallback.

**Konsequenz**: Alle 12 kritischen Tools migriert. Kein Tool kann mehr den Server hängen lassen.

---

## ADR-002: Multi-Layer Graph statt pure Import-Edges (2026-05-20)

**Kontext**: Dashboard zeigte ~32% isolierte Nodes. Nur Import-Edges existierten.

**Entscheidung**: 4-Layer System mit gewichteten Edges: Import (1.0), Implicit/Module (0.8), CoChange (0.5), Sibling (0.2). Sibling nur für verbleibende Isolate.

**Konsequenz**: Orphan Rate von ~32% auf 2.4% reduziert. Dashboard zeigt jetzt zusammenhängendes Netzwerk.

---

## ADR-003: Shell Allowlist default populated (2026-05-20)

**Kontext**: Security Audit zeigte: leere Allowlist = alles erlaubt. MCP-Clients könnten beliebige Commands ausführen.

**Entscheidung**: Default mit ~70 sicheren Dev-Tools (git, cargo, npm, ls, grep...). Leere Liste = expliziter opt-out. `#[serde(default = "default_shell_allowlist")]`.

**Konsequenz**: Neue Installationen sind secure-by-default. Bestehende User mit explizit leerer Liste behalten opt-out.

---

## ADR-004: Git TTL-Cache (2026-05-20)

**Kontext**: ctx_impact, ctx_pack, graph_enricher rufen git pro Request auf. Kein Cross-Request-Cache.

**Entscheidung**: `core/git_cache.rs` mit LazyLock Mutex HashMap, TTL 10s (status/diff) / 60s (log). Max 100 Entries mit Eviction.

**Konsequenz**: Wiederholte git-Aufrufe innerhalb einer Session sind ~0ms statt ~50ms. Modul bereit für Integration in Tools.

---

## ADR-005: Proxy /status hinter Auth, /health offen (2026-05-20)

**Kontext**: Beides war ohne Auth erreichbar. /status zeigt PID, Stats (Information Disclosure).

**Entscheidung**: /health bleibt offen (Loadbalancer/Monitoring brauchen es). /status erfordert Bearer-Token.

**Konsequenz**: Minimale Info Exposure. Monitoring-Kompatibilität bleibt erhalten.

---

## ADR-006: env Filtering in ctx_shell (2026-05-20)

**Kontext**: `extra_env` aus MCP-Args wurde ungefiltert an Subprozess weitergegeben. LD_PRELOAD/DYLD_INSERT_LIBRARIES = Code Injection.

**Entscheidung**: Blocklist für gefährliche env keys (LD_PRELOAD, DYLD_*, BASH_ENV, IFS, SHELL, CDPATH). Silent drop.

**Konsequenz**: Böswillige MCP-Clients können keine Library-Injection mehr via env durchführen.

---

## ADR-007: Full Consolidation Pipeline in Production (2026-05-21)

**Kontext**: `consolidate_to_session()` speicherte Provider-Daten nur im Session Cache. `apply_artifacts()` (BM25, Graph, Knowledge) wurde nie aufgerufen. Provider-Daten waren für `ctx_semantic_search`, `ctx_knowledge`, und Cross-Source Hints unsichtbar.

**Entscheidung**: `consolidate_to_session()` und `predict_and_prefetch()` rufen jetzt `apply_artifacts_to_stores()` in einem Background-Thread auf. Synchron → Cache, Asynchron → BM25 + Graph + Knowledge.

**Konsequenz**: Alle Provider-Daten sind First-Class Citizens. `ctx_semantic_search` findet Issues/PRs, `ctx_knowledge` erinnert sich an Bugs/Features, `ctx_read` zeigt Cross-Source Hints.

---

## ADR-008: `auto_index` Default auf `true` (2026-05-21)

**Kontext**: `providers.auto_index` war per Default `false`. Neue User mussten es manuell aktivieren, um Provider-Daten in BM25/Graph/Knowledge zu bekommen.

**Entscheidung**: Default auf `true` geändert. Opt-out via `auto_index = false` für reine Cache-Nutzung.

**Konsequenz**: Neue Installationen profitieren sofort von vollständiger Provider-Integration.

---

## ADR-009: MCP Bridge ID Schema `mcp:<name>` (2026-05-21)

**Kontext**: Alle `McpBridgeProvider`-Instanzen registrierten sich mit dem statischen ID `mcp_bridge`. Mehrere Bridges überschrieben sich gegenseitig.

**Entscheidung**: `Box::leak` Pattern (analog zu `ConfigProvider`) für dynamische `&'static str` IDs: `mcp:knowledge-base`, `mcp:github-issues` etc.

**Konsequenz**: Beliebig viele MCP Bridges können gleichzeitig registriert werden. Doctor zeigt jede Bridge einzeln an.

---

## ADR-010: CORS einzig in der App, nicht am Edge (2026-06-08)

**Kontext**: `api.leanctx.com` hatte zwei CORS-Layer: eine Traefik-Edge-Middleware (`leanctx-api-cors`, Liste nur `leanctx.com`/`www`) und die App (`tower-http CorsLayer`, Liste inkl. `http://localhost:4321`). Die Listen driftete auseinander: Preflight (OPTIONS) wurde am Edge beantwortet → `localhost` bekam kein `Access-Control-Allow-Origin`, echte Requests aber schon. Lokale Website-Entwicklung gegen die API brach still, Produktion lief.

**Entscheidung**: Redundante Traefik-CORS-Middleware entfernt. Die App (`rust/src/cloud_server`) ist alleinige Quelle für erlaubte Origins und behandelt Preflight + echte Requests. Edge-Config als Infra-as-Code versioniert (`lean-ctx-cloud/cloud-infra/traefik-leanctx-com.yml`).

**Konsequenz**: Eine einzige Origin-Liste, kein Drift mehr. Verifiziert: Preflight für `leanctx.com`/`www`/`localhost:4321` liefert ACAO, echte Requests genau 1 ACAO (kein Duplikat), fremde Origins werden abgelehnt.

---

## ADR-011: Team-Pricing ehrlich — ship-now vs. roadmap (2026-06-08)

**Kontext**: Live-Test des Team-Servers (`http_server/team`) bestätigte echten Mehrwert: geteilter Workspace über HTTP, RBAC (Viewer/Member/Admin/Owner), Audit-Log pro Call, Live-SSE-Event-Bus (alles verifiziert). ABER die Pricing-Seite + Account-Dashboard bewarben *managed connectors*, *private registry*, *marketplace revenue share*, *SSO/SCIM* und *hosted* als aktive Team-Features. Diese sind als Produkt nicht eingelöst (nur Flags in `plans.rs`; `marketplace: None`; revenue_share nur Anzeige). Eine $19-Zahlung provisioniert heute nichts automatisch.

**Entscheidung**: `pricing.ts` trennt gelieferte Features von einer klar markierten `roadmap`-Liste; `PricingPage.astro` rendert „On the roadmap" mit Uhr-Icon; Hosting als „managed setup (beta)"; `AccountBillingPage` badged nicht-live Entitlements mit „soon". No-vaporware: nie für etwas abrechnen, das nicht liefert.

**Konsequenz**: Verkaufbar ohne Falschversprechen. Real beworben: geteilter, auditierter Team-Kontext + RBAC + Event-Bus. Roadmap: hosted index/quota, managed connectors, private registry, revenue share, SSO/SCIM. Commit `946739836` (deploy/GitLab).

---

## ADR-012: Managed-Team via Provisioning-Bridge mit manual-beta Fallback (2026-06-08)

**Kontext**: Der Team-Server ist self-host-fertig, aber das verkaufte *managed* ($19) existierte nicht. Coolify v4 läuft auf fitim (API `:8000`, Traefik v3.6). Das Billing-Service (`lean-ctx-cloud`) hat bereits Webhook→`billing_subscriptions`.

**Entscheidung**: Bridge im privaten `lean-ctx-cloud` (nie GitHub). Subscription active → `billing_team_instances` (Zustandsmaschine `pending_provision→active→suspended→deprovisioned`) + Owner-Token (nur Hash gespeichert). Auto-Provision via Coolify-API wenn `COOLIFY_API_TOKEN` gesetzt; sonst `pending_provision` für manuelles Ops-Fulfillment = **echter** managed-beta-Pfad (kein Stub), deckungsgleich mit der ausgelieferten Copy. Quota/Seats aus `plans.rs` (SSOT). Design: `docs/team-provisioning-bridge-v1.md`.

**Konsequenz**: Schritte 1–3 (Schema, Webhook-Wiring, Token+Edge+Dashboard) sofort baubar → sellable managed-beta. Schritt 4 (Coolify-Auto-Provision) braucht nur einen `COOLIFY_API_TOKEN`. GitLab Epic #1, Tickets #2–#9.

## ADR-013: Solo Pro / Personal Cloud — bezahlter `cloud_sync`-Tier, `pro` wird eigener Plan (2026-06-08)

**Kontext**: Die einzige Solo-Revenue-Linie war Supporter (recognition-only; `pro`/`sponsor` waren Aliase auf `Plan::Supporter`). Es fehlte ein bezahlter Solo-Capability-Tier. Das Backend (Account, Auth, 7×`/api/sync/*`, Checkout, Webhook) existierte bereits → nur Packaging + Gating nötig.

**Entscheidung**: Eigener `Plan::Pro` ($9/Mo · $90/Jahr) mit additivem `cloud_sync`-Entitlement (`free ⊂ supporter ⊂ pro ⊂ team ⊂ enterprise`). `pro` parst jetzt auf `Plan::Pro`; nur `sponsor` bleibt Supporter-Alias (sicher, da keine echten `pro`-Subs existieren, TEST-Mode). Die 7×`/api/sync/*`-Endpoints werden via `cloud_server/billing_edge.rs::require_cloud_sync` gated → `402` für Free/Supporter (Hard-Gate für die gehostete API; Self-Hoster-Opt-out `LEANCTX_CLOUD_SYNC_OPEN=1`, öffnet auch automatisch wenn Billing nicht konfiguriert ist). Local-Free-Invariant bleibt intakt (Sync ist ein gehosteter Dienst, keine lokale Capability). CLI: eigenes `lean-ctx cloud upgrade` (da `lean-ctx upgrade` = Self-Updater-Alias); `sync` läuft Free-Stats immer + Pro-Surfaces mit **einem** 402-Upgrade-Hinweis. Website (deploy/GitLab-only): Pro-Card auf `/pricing` + Pro/Team-Checkout auf `/account/billing`.

**Konsequenz**: Schnellster Self-Serve-Revenue-Hebel für Einzelpersonen; additive `billing-plane-v1`-Änderung ohne v2-Bump. Engine `551b414a6` (github+origin), Deploy `b279e17e6` (origin/GitLab). Voller Plan + As-Built: `docs/business/16-solo-pro-personal-cloud.md`. Offen: Stripe-LIVE-Keys (mit dem restlichen Billing).

---

## ADR-014: Customer-owned Enterprise Token-Control-Platform (2026-07-17)

**Entscheidung**: LeanCTX wird als Data, Control und Value/Evidence Plane im
Kundennetzwerk betrieben. Thinkery monetarisiert Implementierung, Enterprise
Subscription, Support und verifizierte Netto-Savings; keine zentrale
Kundendaten-Abhängigkeit.

## ADR-015: OCLA Rust + Wire Contract (2026-07-17)

**Entscheidung**: 14 kleine Traits plus versionierter Wire Contract, Capability
Discovery, Contract Suite und externer Consumer. P7 ist required.

## ADR-016: Honest Savings Evidence (2026-07-17)

**Entscheidung**: Signatur beweist Integrität/Herkunft. Messmethode, Qualität,
exklusive Attribution, Approval und Vertrag bestimmen abrechenbare Savings.

## ADR-017: OSS verkauft Trust; Commercial verkauft Skalierung (2026-07-17)

**Entscheidung**: lokale Kernfunktion, OCLA, Wire, Gateway, Ledger/Verifier und
Basis-LEARN/ACT bleiben OSS. Enterprise Control Plane, Value Gate, Assurance,
LTS, Compliance, SLA und Services sind commercial.

## ADR-018: Canonical Token Envelope (2026-07-17)

**Entscheidung**: OCLA/Control nutzen providerneutrale Request-, Stream-, Tool-,
Usage- und Error-Typen. Adapter melden unsupported/lossy explizit; Golden Traces
verhindern semantischen Drift.

## ADR-019: W0–W10 steuert Gesamtprogramm (2026-07-17)

**Entscheidung**: P0–P11 bleiben technische OCLA Work-Packages. Security/SRE,
Provider Fabric, Commercial Readiness, Pilot und GA besitzen eigene Waves und
blockierende Evidence Gates.

## ADR-020: AI Value Gate ist Commercial Surface (2026-07-17)

**Entscheidung**: LeanCTX bleibt Plattformmarke und offene Data Plane. AI Value
Gate verkauft Org Control, Value, Assurance und Settlement. Org Gateway bleibt
offener Run-Mode; Kundendaten bleiben customer-owned.

## ADR-021: Enterprise Pricing negotiated (2026-07-17)

**Entscheidung**: Beispielpreise, Prozente und Garantien sind Hypothesen. Nur
das Customer Schedule definiert Baseline, Quality, Cap/Floor und Settlement.

## ADR-022: Repository/Delivery Boundary (2026-07-17)

**Entscheidung**: GitHub `yvgude/lean-ctx` ist kanonischer Apache-2.0 Source;
GitLab `root/lean-ctx` wird read-only Mirror. AI Value Gate, Cloud, Deployment
Factory und Customer Overlays bleiben getrennte private Repositories. Private
Services konsumieren nur versionierte OSS Contracts. Production nutzt required
CI, SBOM, Build-Provenance, Signatur und immutable OCI Digests. W0 beinhaltet
Secret-Rotation, Public-History-Audit, Branch Protection und Cloud-CI.
