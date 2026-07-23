# Context Cockpit — Architecture Audit & Premium Roadmap

Stand: 2026-07-23

## Bestandsaufnahme

| Kategorie | Dateien | Grösse |
|---|---|---|
| Components (JS) | 23 | 518 KB |
| Libraries (JS) | 6 | 61 KB |
| Styles (CSS) | 1 | 89 KB |
| HTML (index) | 1 | 49 KB |
| Backend (Rust) | 1 | 1026 LOC |
| Tests (JS) | 4 | 16 KB |
| **Gesamt** | **36** | **733 KB** |

### Top-5 grösste Components

| Component | KB | Zweck |
|---|---|---|
| `cockpit-graph.js` | 79 KB | Call-Graph + Dependency-Visualisierung (D3) |
| `cockpit-live.js` | 52 KB | Live-Activity-Feed |
| `cockpit-context.js` | 49 KB | Context-Window-Inhalt |
| `cockpit-roi.js` | 36 KB | ROI & Savings-Dashboard |
| `cockpit-overview.js` | 32 KB | Home/Overview (Zusammenfassung) |

---

## Kritische Findings

### F1 — Overview ist ein Mega-Duplikator

`cockpit-overview.js` (32KB) konsumiert **6 APIs** die auch von anderen Views konsumiert werden:

| API | Auch in |
|---|---|
| `/api/stats` | live, remaining, roi |
| `/api/roi` | roi |
| `/api/gain` | remaining |
| `/api/context-triage` | commander |
| `/api/spend` | roi |
| `/api/session` | context |

**Verdict**: Overview zeigt Zusammenfassungen von Daten die in dedizierten Views detailliert werden. Das ist akzeptabel als "Home", aber die 32KB sind zu viel — vieles davon ist redundante Render-Logik.

### F2 — Context + Commander + Compression = 3 Views für dasselbe Thema

- **context.js** (49KB): "Was ist im Context-Window?"
- **commander.js** (25KB): "Context Triage — was soll raus?"
- **compression.js** (20KB): "Compression Lab — was wurde gespart?"

Alle drei drehen sich um **Context-Window-Management**. Commander und Context teilen sogar denselben `/api/context-overlay` Endpoint (3× aufgerufen!).

**Verdict**: → **MERGE** zu einer einzigen "Context" View mit 3 Tabs (Contents | Triage | Compression). **Spart ~94KB → ~40KB**.

### F3 — Knowledge + Memory + Search = 3 Views für "Was weiss lean-ctx?"

- **knowledge.js** (32KB): "Facts, Learnings"
- **memory.js** (13KB): "Episodes, Procedures, Bug Memory"
- **search.js** (17KB): "Search indexed files, symbols"

Knowledge und Memory sind konzeptionell dasselbe — "persistent AI Memory". Search ist die Abfrage-Seite davon.

**Verdict**: → **MERGE** zu "Knowledge" mit 2 Tabs (Memory | Search). **Spart ~62KB → ~30KB**.

### F4 — ROI + Remaining + Leaderboard = 3 Views für "Savings"

- **roi.js** (36KB): "Signed, verifiable savings"
- **remaining.js** (24KB): "Budget remaining" — **irreführender Name**, ist eigentlich Savings-Analyse
- **leaderboard.js** (25KB): "Community Leaderboard"

ROI und Remaining teilen `/api/stats` und `/api/gain`. Leaderboard (25KB!) ist ein **Community-Feature** das nichts mit der Kernfunktion zu tun hat.

**Verdict**: → **MERGE** ROI + Remaining zu "Savings". Leaderboard als separates **optionales** Feature behalten aber in der Nav zurückstufen. **Spart ~85KB → ~40KB**.

### F5 — Health + Protection = 2 Views für "Ist alles OK?"

- **health.js** (21KB): "Guards, Verification, Anomalies"
- **protection.js** (8KB): "Risk & Policies, OWASP Coverage"

Beide prüfen "System-Gesundheit" aus verschiedenen Winkeln. Keiner nutzt `/v1/kernel/*` APIs.

**Verdict**: → **MERGE** zu "System Health" mit 2 Tabs (Guards | Security). **Spart ~29KB → ~18KB**.

### F6 — Graph + Architecture + Explorer = 3 Views für Code-Struktur

- **graph.js** (79KB!!!): D3 Call-Graph — **grösste Datei im ganzen Cockpit**
- **architecture.js** (6KB): Module-Dependencies
- **explorer.js** (13KB): File/Symbol-Browser — **Agent 04 fand: Element registriert aber nie geladen, zeigt permanent "Loading..."**

79KB für einen Call-Graph ist **absurd**. Das ist mehr als manche ganze Web-Apps.

**Verdict**: → **MERGE** zu "Code Structure" mit 2 Tabs (Dependencies | Call Graph). Explorer **REMOVE** (defekt). Graph.js **SIMPLIFY** (Ziel: <20KB). **Spart ~98KB → ~30KB**.

### F7 — Kein einziger Consumer von `/v1/kernel/*` APIs

Die 7 Kernel-API-Endpoints (R31-R33) werden **null mal** im Cockpit genutzt:
- `/v1/kernel/dashboard` — nicht genutzt
- `/v1/kernel/etpao` — nicht genutzt
- `/v1/kernel/config` — nicht genutzt
- `/v1/kernel/evidence` — nicht genutzt
- `/v1/kernel/health` — nicht genutzt
- `/v1/kernel/report` — nicht genutzt
- Provider Stats — nicht genutzt

### F8 — Nav-Metadata dreifach dupliziert

Navigation-Labels/Beschreibungen existieren in:
1. `cockpit-nav.js` — `COCKPIT_NAV_SECTIONS`
2. `router.js` — eigene View-Registry
3. `index.html` — separate Mode-Key und Reader

Labels driften bereits: "Settings" vs "Quick Settings".

### F9 — style.css (89KB) unkontrolliert gewachsen

89KB CSS ohne klare Methodik (kein BEM, kein Utility-First). Vermutlich >30% dead CSS.

### F10 — shared.js (26KB) ist eine Grab-Bag

26KB mit gemischten Utilities, Formatierungen, und UI-Helfern die teilweise in format.js (5KB) dupliziert werden.

---

## Premium-Architektur: Ziel

### Von 22 Views → 7 Views

| # | View | Inhalt | Aus (alt) |
|---|---|---|---|
| 1 | **Home** | Status, Key Metrics, Quick Actions | overview |
| 2 | **Context** | Contents, Triage, Compression (3 Tabs) | context + commander + compression |
| 3 | **Knowledge** | Memory, Search (2 Tabs) | knowledge + memory + search |
| 4 | **Savings** | ROI, Budget, Cost per Provider (3 Tabs) | roi + remaining + Kernel Provider Stats |
| 5 | **System** | Guards, Security, Kernel Health (3 Tabs) | health + protection + Kernel APIs |
| 6 | **Code** | Dependencies, Call Graph (2 Tabs) | graph + architecture |
| 7 | **Activity** | Live Feed, Agents, Sessions (3 Tabs) | live + agents + replay |

### Eliminiert (8 Views → REMOVE/ABSORB)

| View | Aktion | Grund |
|---|---|---|
| `commander` | → Context Tab "Triage" | Duplikat von Context |
| `compression` | → Context Tab "Compression" | Duplikat von Context |
| `memory` | → Knowledge Tab "Memory" | Duplikat von Knowledge |
| `search` | → Knowledge Tab "Search" | Duplikat von Knowledge |
| `remaining` | → Savings Tab "Budget" | Irreführend benannt, Duplikat ROI |
| `protection` | → System Tab "Security" | Duplikat Health |
| `explorer` | REMOVE | Defekt (Loading-Screen), Duplikat |
| `leaderboard` | → Savings Tab "Community" (optional) | Nice-to-have, nicht Core |

### Zusätzlich

| Aktion | Bereich |
|---|---|
| `palette` | → In Settings integrieren oder REMOVE |
| `tour` | REMOVE (5KB für Onboarding das niemand braucht) |
| `area-tabs` | Beibehalten (generische Tab-Infrastruktur) |
| `settings` | → In Nav als Panel integrieren |

### Erwartete Einsparungen

| Metrik | Vorher | Nachher (Ziel) |
|---|---|---|
| Views | 22 | 7 |
| JS Components | 23 | 7 + 2 (shared: area-tabs, nav) |
| JS Grösse | 518 KB | ~200 KB |
| CSS | 89 KB | ~40 KB |
| Kernel-API Integration | 0 | 7 Endpoints |

---

## Umsetzungs-Phasen

### Phase C1 — Context Consolidation
**Scope**: Merge context + commander + compression → "Context" (3 Tabs)
**Aufwand**: Mittel (94KB analysieren, ~40KB Ziel)
**Priorität**: HOCH (grösste Duplikation)

### Phase C2 — Savings & ROI Consolidation
**Scope**: Merge roi + remaining → "Savings", Kernel Provider Stats Tab
**Aufwand**: Mittel (60KB → ~35KB)
**Priorität**: HOCH (Business-Value durch Provider-Cost-Integration)

### Phase C3 — Knowledge Consolidation
**Scope**: Merge knowledge + memory + search → "Knowledge" (2 Tabs)
**Aufwand**: Mittel (62KB → ~30KB)
**Priorität**: MITTEL

### Phase C4 — System Health + Kernel Integration
**Scope**: Merge health + protection → "System", Kernel-API Integration
**Aufwand**: Klein (29KB → ~18KB + Kernel-Anbindung)
**Priorität**: MITTEL (macht Kernel-Arbeit R31-R33 sichtbar)

### Phase C5 — Code Structure Cleanup
**Scope**: Graph.js von 79KB auf <20KB, architecture integrieren, explorer REMOVE
**Aufwand**: HOCH (79KB refactoren)
**Priorität**: NIEDRIG (funktional, nur zu gross)

### Phase C6 — Activity Consolidation
**Scope**: Merge live + agents + replay → "Activity" (3 Tabs)
**Aufwand**: Mittel (84KB → ~45KB)
**Priorität**: NIEDRIG

### Phase C7 — Infrastructure Cleanup
**Scope**: Nav-Metadata SSOT, CSS Audit (89KB → ~40KB), shared.js Cleanup
**Aufwand**: Klein-Mittel
**Priorität**: MITTEL (nach View-Consolidation)

### Phase C8 — Home Refresh
**Scope**: Overview.js auf ~15KB reduzieren, Kernel-Metriken integrieren
**Aufwand**: Klein
**Priorität**: NIEDRIG (nach allen Consolidations)

---

## Prinzipien für Premium-Cockpit

1. **7 Views, nicht 22** — jede View hat einen klaren Zweck
2. **Tabs statt Views** — zusammengehörige Daten als Tabs, nicht separate Seiten
3. **Ein API-Call pro Datensatz** — keine doppelten Fetches desselben Endpoints
4. **Kernel-APIs integriert** — Provider Stats, Health, Evidence sichtbar
5. **Nav-Metadata SSOT** — eine Quelle für Labels, Beschreibungen, Routing
6. **< 250KB total JS** — von 518KB auf ~200KB
7. **< 50KB CSS** — von 89KB auf ~40KB
8. **Keine defekten Views** — Explorer REMOVE, Tour REMOVE
