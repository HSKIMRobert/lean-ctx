# Org SSO (OIDC) v1 (GL #482)

Self-serve OIDC single sign-on for orgs: members authenticate against the
org's identity provider (Okta, Entra ID, Google Workspace, any spec-compliant
OIDC OP) and receive the same account session a password login produces.
SAML and SCIM are out of scope for v1 (deferred to the full #398).

## Roles of the two planes

| Plane | Repo | Responsibility |
|---|---|---|
| Control plane | `lean-ctx-cloud` (private) | System of record: `billing_org_sso` config (secret sealed via AEAD), DNS-TXT domain verification, JIT membership, `billing_sso_audit` trail |
| Edge | `lean-ctx` OSS `cloud_server::sso` | OIDC Relying Party: Authorization Code + PKCE, discovery/JWKS cache, ID-token verification, session + handoff |

## Login flow

```
login page ── POST /api/auth/sso/start {email}
            ◄─ {sso:false}                      (no verified IdP → password UI)
            ◄─ {sso:true, redirect_url}         (state+nonce+PKCE stored, 10-min TTL)
browser ──► IdP authorize … ──► GET /api/auth/sso/callback?code&state
   edge: consume state (single use) → fetch client secret (per login, never cached)
         → token exchange (code + PKCE verifier)
         → verify ID token: JWKS signature, iss, aud, exp, nonce
         → email claim must be under the org's verified domain
         → JIT user (passwordless, email pre-verified) + JIT org membership
         → mint api key + one-time handoff code (60-s TTL)
browser ◄─ 302 leanctx.com/login/?sso_handoff=…
login page ── POST /api/auth/sso/handoff {code} ─► {api_key, user_id, email}
```

Failures redirect to `/login/?sso_error=<reason>` with neutral reasons
(`idp_denied`, `expired`, `missing_params`, `verify_failed`, `unavailable`).
Detailed causes go to server logs only.

## Security invariants

1. **Domain proof before login**: a config answers lookups only after the
   DNS-TXT verification (`_leanctx-sso.<domain>` =
   `leanctx-sso-verify=<token>`); checked via DoH (Cloudflare, then Google).
   `UNIQUE(email_domain)` prevents cross-org domain claims.
2. **Asserted email ⊂ verified domain**: an IdP can only sign in addresses
   under the domain its org proved to own — checked again at the callback.
3. **No symmetric algs**: ID tokens signed with HS*/none are rejected before
   signature evaluation (alg-confusion defense). RS256/384/512, PS256/384/512,
   ES256/384 accepted.
4. **Nonce binding**: the ID token's `nonce` must equal the per-flow stored
   nonce; states are single-use (`DELETE … RETURNING`) with 10-min freshness.
5. **PKCE everywhere** (S256) — even though a confidential client secret is
   also used.
6. **Secrets**: client secret sealed at rest (ChaCha20-Poly1305, same scheme
   as connector credentials), decrypted per token exchange, never persisted
   or cached on the edge; api keys never appear in URLs (one-time handoff,
   60-s TTL, single use).
7. **Break-glass**: `sso_required` never applies to the org owner's email —
   a broken IdP cannot lock the org out of its own dashboard. Password
   logins/registrations for enforced domains are refused *before* credential
   checks (no account-existence oracle).
8. **email_verified:false** from the IdP rejects the login; an absent claim
   is accepted (the IdP authenticated the user).

## Entitlement

SSO config is allowed for Team and Enterprise plans (checked at
`PUT …/sso`). The upcoming Business tier inherits the gate; existing Team
configs are grandfathered at the price move.

## API surface

### Edge (api.leanctx.com)

| Route | Auth | Purpose |
|---|---|---|
| `POST /api/auth/sso/start` | none (email body) | begin flow → `{sso, redirect_url}` |
| `GET /api/auth/sso/callback` | IdP redirect | exchange + verify → 302 with handoff |
| `POST /api/auth/sso/handoff` | one-time code | `{api_key, user_id, email}` |
| `GET/PUT/DELETE /api/account/org/sso` | account bearer | owner config proxy |
| `POST /api/account/org/sso/verify` | account bearer | DNS-TXT check |
| `PUT /api/account/org/sso/required` | account bearer | enforcement toggle |

### Control plane (private, X-Internal-Key)

| Route | Purpose |
|---|---|
| `GET/PUT/DELETE /api/billing/org/{user_id}/sso` | owner-gated config CRUD |
| `POST /api/billing/org/{user_id}/sso/verify` | DoH TXT verification |
| `PUT /api/billing/org/{user_id}/sso/required` | enforcement (verified only) |
| `GET /api/billing/sso/lookup/{domain}` | public config half + owner email |
| `GET /api/billing/sso/exchange/{domain}` | + decrypted secret (per exchange) |
| `POST /api/billing/sso/jit` | ensure membership after login |

## Storage

- `billing_org_sso(org_id PK→billing_orgs, email_domain UNIQUE, issuer,
  client_id, client_secret_enc, domain_verify_token, domain_verified_at,
  sso_required, …)`
- `billing_sso_audit(id, org_id, event, detail, created_at)` — append-only
  (`sso_config_updated`, `sso_domain_verified`, `sso_required_changed`,
  `sso_config_deleted`, `sso_login`). Read model lands with #400.
- Edge: `sso_login_states(state_sha256 PK, email_domain, nonce,
  pkce_verifier, created_at)` and `sso_handoff_codes(code_sha256 PK, user_id,
  api_key, email, created_at)` — both swept opportunistically, both
  hash-keyed.
