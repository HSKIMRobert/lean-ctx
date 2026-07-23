# 11 — Core vs. Enterprise: die im Code ERZWUNGENE Grenze

> Antwort auf "ich hab Angst, dass wir zu viel offen machen": Die Grenze ist **keine Konvention** — sie ist **heute schon maschinell (CI) erzwungen** durch 4 Mechanismen. "Zu viel offen" kann nicht versehentlich passieren, weil jedes neue Feature **klassifiziert werden MUSS** (sonst rot) und commercial-Code physisch getrennt + push-gesperrt ist. Code-Stand 2026-06-30.

## 1. Die 4 Enforcement-Layer (mit Datei:Zeile)

### Layer 1 — Klassifikations-SSOT
[rust/src/core/server_capabilities.rs](../../../rust/src/core/server_capabilities.rs) hat **drei** Const-Arrays + `features()`:
- `LOCAL_ALWAYS_ON_FEATURES` (Zeile 82): immer `true`, nie an Account/Lizenz/Plan gebunden.
- `LOCAL_OPTIONAL_FEATURES` (Zeile 95): frei, nur per **Cargo-Feature** (Compile-Time) gated.
- `COMMERCIAL_PLANE_FEATURES` (Zeile 104): additiv, opt-in per Cargo-Feature.
- `features()` (Zeile 108): LOCAL_ALWAYS_ON hartkodiert `true`; Rest `cfg!(feature = ...)`.

### Layer 2 — Invariant-Tests (Build bricht)
[rust/tests/local_free_invariant.rs](../../../rust/tests/local_free_invariant.rs):
- `local_plane_is_default_and_free` — `plane=="personal"` + alle LOCAL_ALWAYS_ON `true`.
- `local_and_commercial_planes_are_disjoint` — kein Commercial-Key in einer Local-Liste.
- `local_features_are_unaffected_by_license_or_plan_env` — `LEAN_CTX_LICENSE/PLAN/ACCOUNT="expired"` ändert KEIN local feature.
- `billing_plane_never_gates_a_local_feature` — fuer ALLE `Plan::all()` × (LOCAL_ALWAYS_ON ∪ LOCAL_OPTIONAL): `entitlement_allows == true`.

Plus in `server_capabilities.rs` der Partition-Test `feature_keys_partition_into_local_and_commercial` (Zeile 215): **jeder** Key in `features()` MUSS in genau der Vereinigung der drei Arrays liegen. Neues Feature ohne Klassifikation → **rot**.

### Layer 3 — Entitlement-Gate (nur 9 commercial Keys)
[rust/src/core/billing/plans.rs](../../../rust/src/core/billing/plans.rs) `entitlement_allows` (Zeile 247):
```text
if LOCAL_ALWAYS_ON -> return true
match feature { private_registry, sso_oidc, sso_scim, revenue_share, supporter,
                cloud_sync, managed_connectors, hosted_index, audit_retention }
_ => true   // alles andere ist lokal (fail-open zugunsten des Users)
```
Nur diese **9** Strings sind ge-gated. Alles andere ist lokal-frei. Kommentar in [rust/src/core/billing/mod.rs](../../../rust/src/core/billing/mod.rs) (Zeile 20): "The local engine has **no entitlement checks**".

### Layer 4 — Physische Trennung (kein Leak auf GitHub)
- **Cargo-Features** ([rust/Cargo.toml](../../../rust/Cargo.toml)): `cloud-server` ist **NICHT** in `default`; `cloud_server`-Modul ist `#[cfg(feature = "cloud-server")]`. Local-Pfad (tools/proxy/engine) kompiliert ohne commercial code.
- **`.github-ignore`** + `.githooks/pre-push` + CI `.github/workflows/security-check.yml` (`PRIVATE_PATHS` + `COMMERCIAL_PATHS` Existenz-Check): proprietäre Dateien (`rust/src/core/license/`, `billing/success_fee.rs`, `billing/stripe_invoice.rs`, `cli/license_cmd.rs`, `docs/business/`, ...) können **nicht** nach GitHub gepusht werden. Diese Dateien sind bereits aus dem OSS-Engine **entfernt** (leben in `lean-ctx-cloud`).

### Runtime-Realität (per Grep bestätigt)
`entitlement_allows`/`require_cloud_sync`/`resolve_plan` werden **nur** in `cloud_server/` (+ CLI-Upgrade-Hints) aufgerufen. In `rust/src/tools/`, `rust/src/proxy/`, `rust/src/engine/`, `rust/src/core/` (ausser `billing/`): **0 Treffer**. Der lokale Pfad ist ungated — by construction.

## 2. Aktuelle Klassifikation (real, aus dem Code)

- **LOCAL_ALWAYS_ON (8)**: `compression`, `caching`, `knowledge`, `session`, **`gateway`**, `sensitivity_floor`, `savings_ledger`, `audit_trail`.
- **LOCAL_OPTIONAL (4, Cargo-gated)**: `ast_compression`, `semantic_search`, `http_server`, `wasm_runtime`.
- **COMMERCIAL_PLANE (2, Cargo opt-in)**: `team_server`, `cloud_server`.
- **Entitlement-Keys (9, nur Hosting/Org/Scale)**: `private_registry`, `sso_oidc`, `sso_scim`, `revenue_share`, `supporter`, `cloud_sync`, `managed_connectors`, `hosted_index`, `audit_retention`.
- **Plan-Matrix**: Free/Supporter/Pro = 1 Seat; Team = 25 (kein SSO im Katalog); Business = 50 + `sso_oidc` + 365d Audit ($149/mo flat); Enterprise = unbounded + `sso_scim` + 3650d Audit (verhandelt).

## 3. Direkt relevant: das Gateway ist bereits local-free

`"gateway"` steht in `LOCAL_ALWAYS_ON_FEATURES` (`server_capabilities.rs:87`). Heisst: Der Gateway als **lokale Engine-Fähigkeit** ist per maschinell erzwungenem Vertrag **frei** — unser M1-Gateway-Mode landet also korrekt im Core. Das bestätigt den Build-Plan, ohne die Philosophie zu verletzen.

## 4. Die Bau-Regel ("so wird es gebaut")

### Für ein LOCALes Feature (Core, frei)
1. Key in `features()` ([server_capabilities.rs](../../../rust/src/core/server_capabilities.rs)) ergänzen → `true` (always-on) oder `cfg!(feature=...)` (compile-optional).
2. Key in **genau eine** Local-Liste (`LOCAL_ALWAYS_ON_FEATURES` oder `LOCAL_OPTIONAL_FEATURES`).
3. **Niemals** `entitlement_allows` im local-Pfad (`tools/`/`proxy/`/`engine/`) aufrufen.
4. `cargo test` grün halten (Partition + Invariant).

### Für ein COMMERCIALes Feature (Enterprise-Plane, bezahlt)
1. Capability-Key in `features()` + in `COMMERCIAL_PLANE_FEATURES`.
2. Eigenes **Cargo-Feature** anlegen; Modul mit `#[cfg(feature = "...")]` gaten (Muster `cloud-server`).
3. Entitlement-Key in `entitlement_allows` + Feld in `Entitlements` + Werte in der Plan-Matrix ergänzen (Katalog bleibt im OSS public + golden-fixture-gepinnt).
4. Enforcement **nur** in `cloud_server`/`lean-ctx-enterprise` (Muster `require_cloud_sync` → HTTP 402), **nie** im local-Pfad.
5. Money-Code (License/Stripe/Success-Fee) → `.github-ignore` + privates Repo.

## 5. Unser Build → Klassifikation (verbindlich, "so gebaut")

| Komponente (M-Phase) | Capability-Key | Bucket | Cargo-Feature | Repo | Enforcement-Punkt |
|---|---|---|---|---|---|
| Gateway-Mode bind+auth (M1) | `gateway` (existiert) + `http_server` | LOCAL | `http-server` (default) | lean-ctx (OSS) | keiner (local-frei) |
| Identity/Projekt-Tag (M1) | `gateway`/metering (kein neuer Gate) | LOCAL | — | lean-ctx (OSS) | keiner |
| Usage-Store self-host (M1) | `savings_ledger`/`audit_trail` | LOCAL | — | lean-ctx (OSS) | keiner |
| Routing + Foundry (M1) | **neu** `routing` | LOCAL_ALWAYS_ON | — | lean-ctx (OSS) | keiner (eigene Rechnung senken = local-frei) |
| Per-Instance-Dashboard (M1) | `http_server` | LOCAL | `http-server` | lean-ctx (OSS) | keiner |
| Shape-Translation (M2) | **neu** `shape_translation` | LOCAL_OPTIONAL | neu `shape-xlat` | lean-ctx (OSS) | keiner |
| SSO/SCIM @800 (M2) | `cloud_server` | COMMERCIAL | `cloud-server` | lean-ctx-enterprise | `sso_oidc`/`sso_scim` (402) |
| Org-weit ERZWUNGENE Budgets (M2) | **neu** `org_budgets` | COMMERCIAL | neu `enterprise` | lean-ctx-enterprise | neuer Entitlement-Key (402) |
| Multi-Tenant Org-Konsole (M2) | **neu** `org_console` | COMMERCIAL | neu `enterprise` | lean-ctx-enterprise | neuer Entitlement-Key (402) |
| License/Stripe/Success-Fee (M2) | — (Money-Mechanik) | COMMERCIAL | — | privat (cloud/enterprise) | `.github-ignore` + CI-Guard |

Lesart: **M1 ist absichtlich fast komplett Core** — aber das ist genau der Commodity-Teil (Routing/Metering/Kompression), dessen Offenheit dich nichts kostet (LiteLLM verschenkt es). Das **Verkaufbare** (SSO/SCIM, erzwungene Org-Budgets, Org-Konsole, Money) sitzt geschlossen in M2 / `lean-ctx-enterprise` — hinter Cargo-Feature + Entitlement + `.github-ignore`.

## 6. Wie wir "zu viel offen" mechanisch verhindern

- Ein neues Feature ohne Klassifikation → **roter Test** (Partition).
- Ein local-Feature versehentlich hinter Plan/Lizenz → **roter Test** (`billing_plane_never_gates_a_local_feature` + env-Test).
- Commercial-Code versehentlich nach GitHub → **CI-/pre-push-Block** (`security-check.yml`).
- Local-Pfad ruft Entitlements → Review-Regel (heute 0 Treffer; als PR-Check verankerbar).

## 7. Akzeptanzkriterium pro Ticket (in den Build-Plan aufgenommen)

Jedes Gateway-Ticket gilt erst als "done", wenn:
1. die betroffene Capability in `server_capabilities.rs` klassifiziert ist,
2. `cargo test` (inkl. `local_free_invariant`) grün ist,
3. commercial Teile cfg-gated + entitlement-keyed + im privaten Repo liegen (nicht im OSS-Pfad),
4. kein `entitlement_allows` im local-Pfad steht,
5. **der Vision-Fit-Scope-Gate erfuellt ist** (genau eine Achse SEE/ROUTE/REMEMBER/PROVE; nicht auf der Out-of-Scope-Liste — `12-vision-fit-and-scope.md` §4/§6),
6. die Positionierung mit **PROVE/SEE** fuehrt (Evidence/Effizienz), nicht mit "Gateway".

## 8. Zweite Grenze: Vision-Fit (Scope, nicht nur Open-Core)

`11` schuetzt die **Open-Core-Grenze** (frei vs. bezahlt). Die **Scope-Grenze** ("gehoert das ueberhaupt zu lean-ctx?") liegt in [`12-vision-fit-and-scope.md`](12-vision-fit-and-scope.md): der **SEE/ROUTE/REMEMBER/PROVE**-Gate + die **Out-of-Scope-Liste** (kein Chat-UI, kein LB-/Failover-**Produkt**, kein Provider-Marktplatz, kein Prompt-/Guardrail-/Eval-Produkt, keine Shape-Translation-Headline). Merkbar: `11` = "frei vs. bezahlt", `12` = "in vs. out of scope". Ein Ticket muss **beide** Grenzen bestehen.
