# Team Server Contract v1

GitLab: `#2331`  
Pillar: Context Delivery  
Scope: workspaces, scopes, audit log, dot-path rewrite

## Config (`TeamServerConfig`)

File is JSON.

- `host` (string, required)
- `port` (number, required)
- `defaultWorkspaceId` (string, required)
- `workspaces` (array, required; must include default workspace)
  - `{ id, label?, root }`
- `tokens` (array, required for serve)
  - `{ id, sha256Hex, scopes }`
  - `sha256Hex` is lowercase hex SHA-256 of the plaintext token
- `auditLogPath` (path, required)
- `disableHostCheck` (bool, default false)
- `allowedHosts` (string[], default [])
- `maxBodyBytes` (number, default 2097152)
- `maxConcurrency` (number, default 32)
- `maxRps` (number, default 50)
- `rateBurst` (number, default 100)
- `requestTimeoutMs` (number, default 30000)
- `statefulMode` (bool, default false)
- `jsonResponse` (bool, default true)

## Workspace selection

Workspace is selected deterministically via:

1. Header `x-leanctx-workspace` (if present and valid)
2. Otherwise `defaultWorkspaceId`

`POST /v1/tools/call` also accepts `workspaceId` in the JSON body (takes precedence over the header for that call).

## Dot-path rewrite (`rewrite_dot_paths`)

For arguments keys `path`, `target_directory`, `targetDirectory`:

- if value is `""` or `"."`, it is rewritten to the workspace root path before executing the tool.

## Scopes

Scope enforcement is tool/action-aware. Tokens must include required scopes for the requested tool.

Errors:

- `401 unauthorized` (missing/invalid token)
- `403 scope_denied` (token lacks required scopes)
- `400 unknown_workspace`

## Audit log (JSONL)

Audit log is JSONL; one object per line:

- `ts` (RFC3339)
- `tokenId`
- `workspaceId`
- `tool`
- `method`
- `allowed` (bool)
- `deniedReason` (string|null)
- `argumentsMd5` (string; MD5 of canonicalized arguments JSON)

Raw arguments are never stored in the audit log.

## Implementation

- `rust/src/http_server/team.rs`
- CLI dispatch: `rust/src/cli/dispatch.rs` (`lean-ctx team ...`)

