# LeanCTX — Konsolidierung & Cleanup (GitLab Ticket-Vorlagen)

Ziel: **Premium-Infrastruktur ohne Feature-Verlust**. Wir konsolidieren nur dort, wo es:
- **Duplikate** gibt (gleiches Feature, zwei Entry-Points),
- **unverdrahtete Bausteine** gibt (Engine existiert, aber kein klarer Pfad),
- **Wartbarkeit**/Testbarkeit verbessert wird, ohne Behavior-Überraschungen.

Prinzipien:
- **Keine neuen Features** (nur Verdrahtung/Polish/Tests/Refactors).
- **Deprecate > Delete** (erst Migration/Compat-Window, dann Removal).
- **Release Gates zuerst**: Tests/CI sichern jede Konsolidierung ab.

---

## EPIC: v3.5.0 “Consolidation” (ohne Feature-Creep)

### Aktueller Status (2026-04-29)
- ✅ **Ticket 1** erledigt: `ctx_callgraph` eingeführt, `ctx_callers`/`ctx_callees` als kompatible Deprecation-Aliase.
- ✅ **Ticket 2** erledigt: `ctx_graph action=diagram` verdrahtet, `ctx_graph_diagram` als Deprecation-Alias.
- ✅ **Ticket 3** erledigt: `ctx_gain action=wrapped` ist primärer Entry-Point; `ctx_wrapped` ist Deprecation-Alias.
- ✅ **Ticket 4** erledigt: produktive `unwrap()`-Treffer in kritischen Pfaden entfernt (`clippy::unwrap_used` auf `--lib` clean).
- ✅ **Ticket 5** erledigt: Golden-/Edge-Tests für `tokens.rs`, `preservation.rs`, `handoff_ledger.rs`, `workflow/` ergänzt und Gates grün.
- ✅ **Ticket 6** erledigt: README enthält klare 3‑Tier Entry Paths (Quick/Power/Enterprise) inkl. Commands und Outcomes.

### Ticket 1 — Tools konsolidieren: `ctx_callers` + `ctx_callees`
- **Problem**: Zwei Tools für spiegelbildliche Queries → unnötige Oberfläche.
- **Ziel**: Ein Tool (`ctx_callgraph`) mit `direction=callers|callees`.
- **Scope**:
  - Neues Tool hinzufügen/umleiten oder bestehendes Tool erweitern
  - Deprecation-Message für alte Tools (1 Release)
  - Tool-Defs + Docs + Tests aktualisieren
- **Non-Goals**: Graph-Engine umbauen.
- **Acceptance Criteria**:
  - `ctx_callers`/`ctx_callees` weiter funktionsfähig (mit Hinweis)
  - Neuer Unified-Path hat identische Ergebnisse
  - `cargo test` + `cargo clippy -- -D warnings` grün
- **Testplan**:
  - Unit: beide Richtungen liefern erwartete Kanten
  - Integration: Tool-Call via Streamable HTTP

### Ticket 2 — `ctx_graph_diagram` in `ctx_graph` integrieren
- **Problem**: Graph-Funktionalität ist verteilt, Nutzer müssen “Tool-Namen kennen”.
- **Ziel**: `ctx_graph action=diagram` erzeugt Diagramm-Ausgabe (Mermaid).
- **Scope**:
  - `ctx_graph` erweitert um `diagram`
  - `ctx_graph_diagram` bleibt 1 Release als Alias
- **Acceptance Criteria**:
  - Diagram-Output identisch zur bisherigen Implementierung
  - Manifest/Tool-Defs konsistent
- **Testplan**:
  - Snapshot-Test auf Output-Format (Mermaid Head + Nodes/Edges)

### Ticket 3 — `ctx_wrapped` Konsolidierung (Duplikat-Analyse + Entscheidung)
- **Problem**: `ctx_wrapped` vs `ctx_gain`/“wrapped” Output — potenzielles Duplikat.
- **Ziel**: Ein klarer Entry-Point, der “Wrapped” liefert, ohne zwei APIs zu pflegen.
- **Scope**:
  - Ist-Analyse: Output/Optionen vergleichen (Parity-Matrix)
  - Entscheidung: Alias + Deprecation oder Merge
- **Acceptance Criteria**:
  - Keine Regression in CLI und MCP
  - Dokumentation zeigt exakt “den” Weg

### Ticket 4 — Unwrap()-Audit (Stabilität + Premium Robustness)
- **Problem**: Viele `unwrap()`-Stellen (u.a. `property_graph`, `pathjail`) → Crash-Risiko.
- **Ziel**: “No-panic-by-default” in kritischen Pfaden.
- **Scope**:
  - Replace `unwrap()` → Fehlerpfad mit Kontext
  - Sentry/Telemetry ggf. mit Error Counters
- **Acceptance Criteria**:
  - Keine `unwrap()` in kritischen IO/Parsing Pfaden (definiert im Ticket)
  - Tests decken Error-Cases ab

### Ticket 5 — Test-Gaps schließen (Release Gate Hardening)
- **Problem**: Wenige harte Gates in `tokens.rs`, `preservation.rs`, `handoff_ledger.rs`, `workflow/`.
- **Ziel**: CI erkennt Wiring-Regressionen früh.
- **Scope**:
  - Je Modul: “golden path” + “edge path” Tests
  - Fokus: deterministische Outputs, keine Netzabhängigkeit
- **Acceptance Criteria**:
  - Neue Tests schlagen fehl, wenn zentrale Invarianten brechen
  - Keine flakiness (kein Timing/Netz)

### Ticket 6 — README “3-Tier Entry Paths” (Quick/Power/Enterprise)
- **Problem**: Einstieg ist für neue User nicht sofort kristallklar.
- **Ziel**: 3 klare Wege inkl. Commands + erwarteter Nutzen.
- **Acceptance Criteria**:
  - Quickstart in <5 Minuten
  - Power-User Path zeigt Graph/Dashboard/Policies
  - Enterprise Path zeigt Governance + Observability

---

## EPIC: v3.4.6 “Premium Wiring & Hygiene” (Restarbeiten)

### Aktueller Status (2026-04-29)
- ✅ **Ticket A** erledigt: Versions-/Tool-Count-Hygiene in README/VISION/Manifest konsolidiert.
- ✅ **Ticket B** erledigt: CHANGELOG enthält release-ready Eintrag für `v3.4.6`.
- ✅ **Ticket C** erledigt: unverdrahtetes Legacy-Modul `watcher.rs` entfernt, Replacement ist der aktive Index-/Graph-Pfad.
- ✅ **Ticket D** erledigt: finale Markdown-Hygiene (`49 MCP Tools` harmonisiert, redundante Root-Drafts entfernt).
- ✅ **Ticket E** erledigt: `LEANCTX_FEATURE_CATALOG.md` als SSOT-Snapshot auf aktuelle Runtime/Oberfläche wiederhergestellt.

### Ticket A — Versions-Hygiene
- **Scope**: Versionsreferenzen in `README.md`, `VISION.md`, Manifests konsistent machen.
- **Acceptance Criteria**: Keine veralteten Versionen in Doku/Manifests.

### Ticket B — CHANGELOG v3.4.6
- **Scope**: Alle Masterplan-Änderungen + neue Wiring-Fixes dokumentieren.
- **Acceptance Criteria**: Changelog ist “release-ready”.

### Ticket C — Orphan/Legacy Module Entscheidung: `watcher.rs`
- **Problem**: Historisch “removed”, aber Codebase enthält Rest/Orphans.
- **Ziel**: Entweder sauber re-verdrahten oder sauber deprecaten + entfernen (mit Begründung).
- **Acceptance Criteria**:
  - Keine “toten” Files ohne klare Status-Doku
  - Wenn deprecated: klarer Replacement Path

### Ticket D — Finaler Markdown Cleanup & Doku-Konsistenz
- **Problem**: Veraltete Tool-Zahlen/Versionen in mehreren Doku-/Paketdateien und redundante ungetrackte Root-Dokumente.
- **Ziel**: Einheitliche, release-taugliche Doku ohne Altlasten.
- **Scope**:
  - Tool-Count-Harmonisierung auf `49 MCP Tools` in FAQ/Skill/Package-Readmes und Package-Metadaten
  - Entfernen redundanter Root-Markdown-Drafts
  - Abschluss-Audit: harte Qualitäts-Gates (`fmt`, `clippy`, `tests`) grün
- **Acceptance Criteria**:
  - Keine aktiven Doku-Hinweise mehr auf 46/48/42 Tool-Counts (ausgenommen historische CHANGELOG-Einträge)
  - Keine unreferenzierten Root-Draft-Markdown-Dateien aus dem Konsolidierungslauf
  - Qualitäts-Gates bleiben grün

### Ticket E — Feature Catalog Restore (SSOT)
- **Problem**: Der Feature-Katalog war als Root-Draft entfernt worden, wird aber als kompakte Feature-Inventur für Releases weiterhin benötigt.
- **Ziel**: Wiederherstellung eines sauberen, aktuellen SSOT-Katalogs mit Tool-Counts, Read-Modes und Deprecation-Map.
- **Scope**:
  - `LEANCTX_FEATURE_CATALOG.md` neu anlegen (Runtime-Snapshot)
  - Quellen explizit auf Manifest/Tool-Defs referenzieren
  - Canonical-vs-Deprecated Entry-Paths klar markieren
- **Acceptance Criteria**:
  - Katalog ist vorhanden, aktuell und konsistent mit `website/generated/mcp-tools.json`
  - Tool-Count = 49 granular / 5 unified
  - Deprecation-Map (`ctx_callers/callees`, `ctx_graph_diagram`, `ctx_wrapped`) dokumentiert

