# Org Audit Log v1 (GL #484)

A unified, append-only governance audit log for orgs, surfaced to the **org
owner** with plan-based retention and CSV export. The reduced, solo-viable
"Long Audit Retention" half of #284 (SSO self-serve shipped separately in
#482). SCIM, SAML, tamper-evident hash chaining and SIEM streaming are out of
scope for v1.

## Planes

| Plane | Repo | Responsibility |
|---|---|---|
| Control plane | `lean-ctx-cloud` (private) | System of record: `org_audit_log`, best-effort writes from every governance path, owner read API + CSV, daily fleet retention sweep |
| Edge | `lean-ctx` OSS `cloud_server::billing_edge` | Owner-bearer proxies: `GET /api/account/org/audit` (+ `/export.csv`) |
| Website | `lean-ctx-deploy` | `/account/audit` — filterable table, relative timestamps, retention notice, CSV download; owner link from the billing org card |

## Storage

```
org_audit_log(
  id          BIGSERIAL PK,   -- stable pagination cursor (id DESC)
  org_id      UUID NOT NULL,
  actor_email TEXT,           -- who (when known)
  event       TEXT NOT NULL,  -- machine event id (snake_case)
  target      TEXT,           -- what/whom (invited role, domain, token id, …)
  detail      TEXT,           -- human-readable extra
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
INDEX (org_id, id DESC)
```

The previous SSO-only `billing_sso_audit` table is migrated into this log and
dropped by a one-shot, idempotent boot migration (guarded by
`information_schema`; the source table is gone after the first run).

## Event vocabulary (v1)

| event | actor | target | emitted by |
|---|---|---|---|
| `sso_config_updated` | — | — | control plane `org_sso::put_org_sso` |
| `sso_domain_verified` | — | domain | `org_sso::verify_org_sso` |
| `sso_required_changed` | — | on/off (detail) | `org_sso::put_org_sso_required` |
| `sso_config_deleted` | — | — | `org_sso::delete_org_sso` |
| `sso_login` | — | email/jit (detail) | `org_sso::sso_jit_membership` |
| `invite_created` | — | label · role | `team::create_invite` |
| `invite_revoked` | — | invite id | `team::revoke_invite` |
| `invite_redeemed` | — | token id · role | `team::redeem_invite` |

All writes are **best effort**: a failed audit insert is logged and swallowed,
never breaking the user-facing operation. The log is append-only — there is no
update or delete API; retention is the only deletion path.

## Retention

The window is the owner-plan's `audit_retention_days` from the
`billing-plane-v1` SSOT (`plan.rs` / open engine `core::billing::plans`):
**Team 90 days, Enterprise 3650, all others 0.** Orgs only exist for Team-and-up
accounts, so the log is effectively a Team+ capability.

Two enforcement paths keep the promise:
1. **Daily fleet sweep** (`audit_retention_job`, also on every boot): for each
   org, delete rows older than its owner-plan window.
2. **On-read sweep**: the owner's read/export first prunes their org to the
   window, so they never see a row older than they're entitled to keep — even
   between daily sweeps.

## API surface

### Edge (api.leanctx.com) — account bearer

| Route | Purpose |
|---|---|
| `GET /api/account/org/audit?before&limit&event` | owner's page (newest first) |
| `GET /api/account/org/audit/export.csv` | owner's CSV (bounded to 5 000 rows) |

`before` is an exclusive `id` cursor, `limit` is clamped to `1..=200`, `event`
is allowlisted to snake_case ids before it reaches the upstream URL.

### Control plane (private, X-Internal-Key)

| Route | Purpose |
|---|---|
| `GET /api/billing/org/{user_id}/audit` | owner-gated page → `{events, retention_days, next_before}` |
| `GET /api/billing/org/{user_id}/audit/export.csv` | owner-gated CSV |

Response page shape:

```json
{
  "events": [
    { "id": 42, "event": "sso_login", "actor": null,
      "target": null, "detail": "email=ada@acme.com jit=existing",
      "at": "2026-06-10T12:00:00+00:00" }
  ],
  "retention_days": 90,
  "next_before": 12
}
```

`next_before` is the cursor for the next page; `null` when the last page was
returned.

## Security / quality invariants

1. **Owner-only.** Reads require the `owner` org role (same posture as SSO
   config). One org's events are never visible to another.
2. **Append-only.** No mutation/delete endpoint; retention is the sole eraser.
3. **Bounded.** Page size clamped (≤200); export bounded (≤5 000 rows);
   pagination is by a stable BIGSERIAL cursor.
4. **No untrusted URL splicing.** The edge allowlists `event` and forwards only
   numeric `before`/`limit`.
5. **Retention is enforced server-side**, not just hidden in the UI — the
   database is swept on a schedule and on read.

## Out of scope (later)

- Plan/billing-change events (Stripe portal already shows billing history).
- SCIM/SAML (full #284/#398), tamper-evident hash chaining, SIEM export.
- Per-member roles beyond owner for read access.
