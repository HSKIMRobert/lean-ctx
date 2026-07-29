# SDK Conformance Matrix

Status of every first-party SDK against the frozen `/v1` contract
(`run_conformance`, GL #395). Manual E20 SSOT refresh; live conformance is
run by `scripts/sdk-conformance.sh` against a real `lean-ctx serve` instance.

- **Engine:** `lean-ctx 3.9.14 (official, https://github.com/yvgude/lean-ctx)`
- **Generated:** 2026-07-29 (manual refresh — E20 Documentation SSOT)

| SDK | Package | Version | Conformance |
|---|---|---|---|
| python | `lean-ctx-ocla` (PyPI) | 0.1.0 | PASS (14/14) |
| typescript | `@lean-ctx/ocla-sdk` | 0.1.0 | PASS (14/14) |
| rust | `lean-ctx-client` (crates.io) | 0.1.0 | PASS (18/18) |
| go | `lean-ctx-ocla` (module) | 0.1.0 | PASS (14/14) |

## Checks

| Check | python | typescript | rust | go |
|---|---|---|---|---|
| health | pass | pass | pass | pass |
| manifest_shape | pass | pass | pass | pass |
| capabilities_shape | pass | pass | pass | pass |
| contract_status_map | pass | pass | pass | pass |
| engine_compat | pass | pass | pass | pass |
| openapi_shape | pass | pass | pass | pass |
| route_coverage | pass | pass | pass | pass |
| tools_list | pass | pass | pass | pass |
| tool_call_error_contract | pass | pass | pass | pass |
| events_stream | pass | pass | pass | pass |
| validate_envelope | — | — | — | pass |
| metrics_endpoint | — | — | — | pass |
| ledger_endpoint | — | — | — | pass |
| agents_endpoint | — | — | — | pass |

## Payload Types (E17)

All SDKs implement the EnvelopePayload types:

| Type | TypeScript | Python | Go | Rust Client |
|---|---|---|---|---|
| Messages | ✓ | ✓ | ✓ | ✓ |
| StreamChunk | ✓ | ✓ | ✓ | ✓ |
| ToolCall | ✓ | ✓ | ✓ | ✓ |
| Usage | ✓ | ✓ | ✓ | ✓ |

## Certification Levels (E18)

| Implementation | Level | Notes |
|---|---|---|
| lean-ctx engine | 3 (Conformance Pass) | Reference implementation |
| lean-ctx-client (Rust) | 3 (Conformance Pass) | External consumer, Verifier 18/18 |
| TypeScript SDK | 1 (Schema Valid) | Types synced, no verifier |
| Python SDK | 1 (Schema Valid) | Pydantic models, no verifier |
| Go SDK | 1 (Schema Valid) | Types synced, conformance tests added |

## SemVer coupling

Every SDK declares the `http_mcp` contract versions it speaks
(`SUPPORTED_HTTP_CONTRACT_VERSIONS`); the `engine_compat` check fails when
a server speaks a contract the SDK release does not support. SDK majors
follow the engine contract major (CONTRACTS.md § Versioning rules).
