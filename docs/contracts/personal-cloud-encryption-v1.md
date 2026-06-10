# personal-cloud-encryption-v1 — Zero-Knowledge Knowledge Vault

Status: **active** (GL #467) · Engine: `core/knowledge_vault.rs` ·
Server: `cloud_server/knowledge.rs` (`knowledge_blobs`)

## Claim

For E2E surfaces, the Personal Cloud backend stores **only ciphertext**. We
cannot read, search, sell or leak knowledge content — provably: the
decryption key is derived from the account API key, of which the server only
ever stores a SHA-256 hash.

## Construction

| Property | Value |
|---|---|
| Cipher | XChaCha20-Poly1305 (AEAD), 24-byte random nonce per seal |
| Key derivation | HKDF-SHA256(salt=`leanctx`, ikm=API key, info=`knowledge-vault-v1`) |
| Domain separation | distinct HKDF `info` per surface — the index-bundle key (`index-bundle-v1`) can never open a vault and vice versa |
| Envelope | `{"v":1,"entries":[{category,key,value},…]}`, serialized then sealed; wire format `nonce ‖ ciphertext` |
| Consistency | whole-account snapshot, last-writer-wins (same model as `hosted-personal-index-v1`) |

The key is identical on every logged-in device (stable API key, not the
rotating OAuth token) — that is what makes cross-device pull work. Key
rotation = new API key + one re-push from any device that has the local store.

## Wire protocol (`/api/sync/knowledge`)

| Request | Behaviour |
|---|---|
| `POST` `Content-Type: application/octet-stream` + `X-Entry-Count: N` | store vault blob (≤ 8 MB), then **delete the account's plaintext `knowledge_entries` rows** — the built-in re-encryption migration |
| `POST` `Content-Type: application/json` | legacy plaintext upserts (deprecated; removed two releases after vault clients ship) |
| `GET` `Accept: application/octet-stream` | encrypted vault blob; `404` when the account has none yet |
| `GET` (anything else) | legacy plaintext listing |

`X-Entry-Count` is a client-declared display metadatum (dashboards show
counts and sizes); the server cannot verify it — by design.

Clients pull vault-first and fall back to the legacy listing on `404` *or*
when an older server ignores the `Accept` header (detected via the response
`Content-Type`).

## What is deliberately NOT E2E

| Surface | Why it stays aggregate/plaintext |
|---|---|
| Commands / CEP / Gain stats | numbers only (counts, token totals) — no content; they feed the savings dashboard and the opt-in leaderboard, which require server-side aggregation |
| Supporter / billing metadata | Stripe-owned, never includes code or knowledge |
| Index bundles | already E2E under `hosted-personal-index-v1` |
| Gotchas endpoint (`/api/sync/gotchas`) | follow-up — same vault pattern, tracked in GL #467 |

## Server obligations

- Zero-content logging: sizes, hashes, counts — never payloads.
- `knowledge_blobs` carries `sha256` over the ciphertext for drift detection.
- The legacy table stays queryable until the deprecation window closes, but
  any vault push purges that account's plaintext rows immediately.
