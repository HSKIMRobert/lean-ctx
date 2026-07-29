# Enterprise Integration Guide — v1

How `lean-ctx-enterprise` consumes the OSS engine over the `/v1` contract
boundary. No path-dependencies, no engine imports.

## Architecture

```
┌─────────────────────────────┐     ┌──────────────────────────┐
│ lean-ctx (OSS)              │     │ lean-ctx-enterprise      │
│                             │     │ (Private)                │
│ Engine, CLI, MCP            │────▶│ Value Gate               │
│ Gateway Admin API           │ /v1 │ SSO/SCIM                 │
│ Settlement Evidence Export  │     │ Org Policy Lifecycle     │
│ OCLA Wire Contract          │     │ Approval/Settlement UI   │
└─────────────────────────────┘     └──────────────────────────┘
```

## Integration Points

| Surface | Endpoint | Consumes | Produces |
|---|---|---|---|
| Evidence Export | GET /api/admin/evidence/export | — | Signed gateway usage evidence |
| OCLA Summary | GET /api/admin/evidence/ocla | — | OCLA-formatted evidence readiness summary |
| Billing Catalog | GET /api/billing/plans | — | Plan catalog (v3) |
| SSO Config | POST /api/auth/sso/configure | OIDC config | SSO session |
| Org Policy | POST /api/policy/org/install | Signed policy pack | Policy active |

`GET /api/admin/evidence` remains a compatible alias for the export endpoint.
The OCLA summary contains `schema_version: 2`, the signed usage export's
period and evidence count, settlement roles present, and explicit missing-role
gaps. It does not turn usage evidence into a settlement approval or invoice.

For a settlement handoff, the private plane obtains a
`SettlementEvidenceManifestV2` and an independently pinned trust store, then
uses the OSS verifier/exporter. The summary is a discovery surface; it is not
a substitute for the v2 manifest or trust input.

## Contract Boundary Rules

1. Enterprise MUST NOT import `lean-ctx` Rust types.
2. All communication uses HTTP JSON over the Wire Contract.
3. Schema versions are backward-compatible and additive only.
4. The OSS engine never gates on plan level (Local-Free Invariant).
5. OSS verifies structure and integrity only; approval, pricing, settlement,
   invoicing, and trust-anchor authority remain private-plane responsibilities.
