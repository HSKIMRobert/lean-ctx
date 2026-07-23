# Context Cockpit — Architecture Audit & Premium Roadmap

Stand: 2026-07-23 (aktualisiert nach C1 Implementation)

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

### F2 — Context + Commander + Compression = 3 Views für dasselbe Thema — **REVIDIERT**

- **context.js** (49KB): "Was ist im Context-Window?"
- **commander.js** (25KB): "Context Triage — was soll raus?"
- **compression.js** (20KB): "Compression Lab — was wurde gespart?"

**C1 Analyse-Ergebnis**: Die Tab-Konsolidierung existiert **bereits**!
- Router `COCKPIT_AREAS` definiert "Context" Area mit 5 Tabs: Triage | Contents | Live | Lab | Settings
- `cockpit-area-tabs.js` rendert die Tab-Leiste automatisch
- Jeder Tab ist ein eigenständiges Web Component (lazy-loaded)
- Kein Merge nötig — die Architektur ist bereits korrekt!

**Verbleibend**: Component-Level Bloat-Reduktion (optional, nicht kritisch).

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
- **explorer.js** (13KB): File/Symbol-Browser — **REVIDIERT: funktional, braucht Tree-Index-Build**

**C1 Analyse-Ergebnis**: Explorer ist **NICHT defekt**:
- Hat Backend `/api/tree` Route (tree.rs)
- `makeViewLoader` + `loadData()` korrekt verdrahtet
- Zeigt "Loading..." nur während Tree-Index im Hintergrund gebaut wird (progressives Loading mit Polling)
- Funktioniert sobald der Index fertig ist

79KB Graph.js: D3 Call-Graph Visualisierung — komplex aber funktional. Slim-Down schwierig ohne Feature-Verlust.

**Verdict**: Explorer **BEHALTEN** (funktional). Graph.js bleibt (D3 braucht Platz). Architecture bereits als eigener Tab in "Project Map" Area integriert.

### F7 — ~~Kein einziger Consumer von `/v1/kernel/*` APIs~~ ✅ BEHOBEN

~~Die 7 Kernel-API-Endpoints (R31-R33) werden **null mal** im Cockpit genutzt.~~

**BEHOBEN in C1 (2026-07-23):**
- Neuer `/api/kernel` Backend-Route: konsolidiert Health, Provider Stats, Evidence, Savings, Subsystem Status
- `cockpit-health.js`: neuer "Kernel" Tab (Health Hero, Savings-Tabelle, Provider Distribution, Subsystem-Grid)
- `cockpit-overview.js`: Kernel-Status-Chip im Home StatusStrip (healthy/degraded/off)
- `cockpit-roi.js`: Provider Distribution Tabelle mit per-Provider Request/Token Breakdown
- Live getestet: `/api/kernel` liefert korrektes JSON mit 6 Subsystemen

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

## Premium-Architektur: Ist-Zustand (nach C1 Analyse)

### Bestehende Area-Struktur (bereits konsolidiert!)

Die Cockpit-Architektur ist **bereits** in 5 Areas mit Tabs organisiert:

| # | Area | Tabs | Zweck |
|---|---|---|---|
| 1 | **Context** | Triage, Contents, Live, Lab, Settings | Was Agents lesen |
| 2 | **Memory** | Knowledge, Episodes, Search, Agents | Was Agents lernen |
| 3 | **Protection** | Guards, Risk & Policies | Sicherheit |
| 4 | **Proof** | ROI, Time Machine, Trends, Leaderboard | Savings beweisen |
| 5 | **Project Map** | Dependencies, Call Graph, Symbols, Explorer, Architecture, Routes | Codebase-Verständnis |

Plus **Home** (Simple Mode: nur Home; Pro Mode: Home + 5 Areas).

### Kernel-Integration (C1 ✅ implementiert)

| Endpoint | Cockpit-View | Status |
|---|---|---|
| `/api/kernel` (neu) | Health > Kernel Tab | ✅ Live |
| `/api/kernel` | Overview > StatusStrip Chip | ✅ Live |
| `/api/kernel` | ROI > Provider Distribution | ✅ Live |

### ~~Von 22 Views → 7 Views~~ — REVIDIERT

Die 5-Area Tab-Architektur existiert bereits. Kein View-Merge nötig.

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

## Revidierte Umsetzungs-Phasen

### Phase C1 — Kernel API Integration ✅ ABGESCHLOSSEN (2026-07-23)
**Deliverables:**
- Neue `/api/kernel` Backend-Route (`rust/src/dashboard/routes/kernel.rs`, 79 LOC)
- Health.js: neuer "Kernel" Tab (Provider Stats, Evidence, Savings, Subsystems)
- Overview.js: Kernel StatusStrip Chip
- ROI.js: Provider Distribution Tabelle
- 27 vorbestehende Clippy-Warnings behoben (21 Dateien, 126 Insertions)
- Release Build ✓, 9051 Tests ✓, Zero Clippy ✓, Live-Test ✓
- Pushed: GitHub ✓ + GitLab ✓

### Phase C2 — Clippy Zero-Warning Policy ✅ ABGESCHLOSSEN (2026-07-23)
**Deliverables:**
- `field_reassign_with_default`, `unchecked_time_subtraction`, `unnecessary_literal_bound`,
  `default_trait_access`, `cloned_ref_to_slice_refs`, `case_sensitive_file_extension_comparisons`,
  `await_holding_lock`, `needless_pass_by_value`, `unnecessary_min_or_max`
- 21 Dateien gefixt, `cargo clippy --all-targets -- -D warnings` passiert

### Phase C3 — CSS Audit + Dead Rule Cleanup
**Scope**: 1616 Zeilen CSS, 733 Rules, geschätzt >30% ungenutzt
**Priorität**: MITTEL

### Phase C4 — Nav-Metadata SSOT
**Scope**: Labels/Descriptions in cockpit-nav.js, router.js, index.html konsolidieren
**Priorität**: NIEDRIG

### Phase C5 — Component-Level Optimierungen
**Scope**: Optional — einzelne grosse Components (Graph 1891L, Live 1227L) refactoren
**Priorität**: NIEDRIG (funktional, Risiko/Nutzen-Verhältnis fragwürdig)

---

## Prinzipien für Premium-Cockpit

1. **5 Areas + Home** — Area-Tab-Architektur bereits korrekt implementiert ✓
2. **Kernel-APIs integriert** — Provider Stats, Health, Evidence sichtbar ✅
3. **Zero Clippy Warnings** — `--all-targets` Policy durchgesetzt ✅
4. **Lazy Loading** — Views laden Daten nur wenn aktiv (via registerLoader) ✓
5. **Nav-Metadata SSOT** — Labels aus einer Quelle (offener Punkt)
6. **CSS Cleanup** — 1616 Zeilen, ~30% vermutlich ungenutzt (offener Punkt)
