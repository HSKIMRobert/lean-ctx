# Publishing Context Packages

How to publish a `.ctxpkg` context package to the hosted registry at
[ctxpkg.com](https://ctxpkg.com) and install packages from it.
Contract: [`ctxpkg-registry-v1`](../contracts/ctxpkg-registry-v1.md).

## One-time setup

1. **Claim your namespace** — log in at leanctx.com, then
   `Account → Registry → Claim namespace`. Lowercase `[a-z0-9-]`, 2–32
   chars. Namespaces are permanent in v1, so pick deliberately.
2. **Mint a publish token** — same page. Tokens look like `ctxp_…`, are
   shown exactly once, and can be revoked anytime (max 10 active).

```bash
export CTXPKG_TOKEN=ctxp_…   # or pass --token per publish
```

## Publish

```bash
# 1. Create your package locally (scoped name = @namespace/name)
lean-ctx pack create --name auth-context --scope @acme --description "Auth service context"

# 2. Export SIGNED — the registry rejects unsigned bundles
lean-ctx pack export @acme/auth-context --sign

# 3. Publish
lean-ctx pack publish acme-auth-context-1.0.0.ctxpkg
```

`--sign` uses an ed25519 key at `~/.lean-ctx/keys/ctxpkg-ed25519.key`
(auto-generated on first use, mode 0600). **This key is your publisher
identity across releases — back it up.** Losing it means future releases
show a different signer key.

Rules the registry enforces at publish time:

- `manifest.name` must equal `@{namespace}/{name}` from the publish target
  and the namespace must be yours (token-bound);
- the ed25519 signature must verify server-side — not just be present;
- versions are SemVer and **immutable**: re-publishing an existing version
  returns 409. Ship a new version instead;
- size cap 8 MiB per artifact.

Every accepted release gets a persisted `trust_report` stating exactly what
was checked (schema, signature, name binding, size) and what was not
(`wasm_capability_audit`, `malware_heuristics`).

## Install

```bash
lean-ctx pack install acme/auth-context          # newest non-yanked version
lean-ctx pack install acme/auth-context@1.0.0    # exact pin
```

The client independently verifies what the registry claims:

1. artifact SHA-256 against the package index,
2. the engine's content-integrity chain (content hash, composite hash,
   byte size),
3. the ed25519 manifest signature, locally.

Each install is pinned in `.lean-ctx/ctxpkg.lock` (commit it):

```toml
[[package]]
name = "@acme/auth-context"
version = "1.0.0"
artifact_sha256 = "…"
registry = "https://ctxpkg.com/api"
```

## Yank

```bash
curl -X DELETE -H "Authorization: Bearer $CTXPKG_TOKEN" \
  https://ctxpkg.com/api/v1/packages/acme/auth-context/1.0.0
```

Yanking excludes a version from `latest` resolution but keeps it
downloadable for reproducibility (installing a pinned yanked version warns
loudly). Nothing is ever deleted.

## Self-hosting / other registries

Both ends honor overrides — useful for air-gapped mirrors or a private
registry speaking the same v1 surface:

```bash
lean-ctx pack publish pkg.ctxpkg --registry https://registry.internal/api
CTXPKG_REGISTRY=https://registry.internal/api lean-ctx pack install acme/auth-context
```
