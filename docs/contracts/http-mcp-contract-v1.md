# HTTP-MCP Contract v1

## Goal

A **versioned HTTP API contract** for lean-ctx Context OS, defining the REST + SSE surface
that sits alongside the Streamable HTTP MCP transport. All endpoints listed below are
served by the same `axum` server that handles MCP protocol messages via fallback routing.

- **workspace-aware**: every request is scoped to a `(workspace_id, channel_id)` pair.
- **observable**: tool calls, session mutations, and graph builds emit events to an SSE bus.
- **redaction-safe**: event payloads are stripped by default; full payloads require Audit scope.
- **bounded**: SSE replay is capped at 1 000 events; rate + concurrency limits protect the server.

## Version (SSOT)

- Runtime (local): `rust/src/http_server/mod.rs`
- Runtime (team): `rust/src/http_server/team.rs`
- Events: `rust/src/core/context_os/context_bus.rs`
- Metrics: `rust/src/core/context_os/metrics.rs`
- Redaction: `rust/src/core/context_os/redaction.rs`

---

## Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | none | Liveness probe (`200 ok`) |
| GET | `/v1/manifest` | bearer | Full MCP manifest |
| GET | `/v1/tools` | bearer | Paginated tool list |
| POST | `/v1/tools/call` | bearer | Execute a single tool |
| GET | `/v1/events` | bearer + `Events` scope | SSE stream with replay |
| GET | `/v1/metrics` | bearer + `Audit` scope | JSON metrics snapshot |
| POST (fallback) | `/*` | bearer | Streamable HTTP MCP transport |

---

## Workspaces and Channels

Every HTTP request is associated with a **(workspace_id, channel_id)** pair that determines
session isolation and event routing.

### Tool Call Requests

Include `workspaceId` and `channelId` in the JSON request body of `POST /v1/tools/call`:

```json
{
  "name": "ctx_read",
  "arguments": { "path": "src/main.rs" },
  "workspaceId": "backend-team",
  "channelId": "feature-auth"
}
```

Both fields default to `"default"` when omitted. Sessions are shared per unique
`(workspace_id, channel_id)` pair — two requests with the same pair share caches,
scratchpad, and knowledge state.

### Workspace Header (Team Server)

The team server supports workspace routing via the `x-leanctx-workspace` HTTP header:

```
x-leanctx-workspace: backend-team
```

The header is resolved during authentication. If the header is absent, the
`defaultWorkspaceId` from the team server configuration is used. An unknown workspace
returns `400 Bad Request`.

### Precedence

| Source | Applies to | Priority |
|--------|-----------|----------|
| `workspaceId` in JSON body | `POST /v1/tools/call` | highest |
| `x-leanctx-workspace` header | all endpoints (team server) | fallback |
| `defaultWorkspaceId` config | team server default | lowest |

---

## Events API (SSE)

### Endpoint

```
GET /v1/events?workspaceId=<ws>&channelId=<ch>&since=<cursor>&limit=<n>
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `workspaceId` | string | `"default"` | Filter events by workspace |
| `channelId` | string | `"default"` | Filter events by channel |
| `since` | i64 | `0` | Cursor — replay events with `id > since` |
| `limit` | usize | `200` | Max events to replay (capped at 1 000) |

### Protocol

Server-Sent Events (SSE) stream with full replay support. The connection starts by
replaying persisted events matching the filter, then switches to live broadcast.

```
HTTP/1.1 200 OK
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive

id: 42
event: tool_call_recorded
data: {"id":42,"workspaceId":"ws1","channelId":"ch1","kind":"tool_call_recorded","actor":"agent","timestamp":"2026-05-05T13:00:00Z","payload":{...}}

id: 43
event: session_mutated
data: {"id":43,"workspaceId":"ws1","channelId":"ch1","kind":"session_mutated","actor":"agent","timestamp":"2026-05-05T13:00:01Z","payload":{...}}
```

### Event Types

| Kind | Trigger |
|------|---------|
| `tool_call_recorded` | Any MCP tool invocation completes |
| `session_mutated` | Shared session state is modified |
| `knowledge_remembered` | Knowledge store entry written |
| `artifact_stored` | Artifact persisted to proof store |
| `graph_built` | Dependency/call graph index built or updated |
| `proof_added` | Evidence ledger entry appended |

### Event Schema (`ContextEventV1`)

```json
{
  "id": 42,
  "workspaceId": "ws1",
  "channelId": "ch1",
  "kind": "tool_call_recorded",
  "actor": "agent",
  "timestamp": "2026-05-05T13:00:00.000Z",
  "payload": { "tool": "ctx_read", "..." : "..." }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | i64 | Monotonically increasing event ID (SQLite autoincrement) |
| `workspaceId` | string | Workspace that produced the event |
| `channelId` | string | Channel within the workspace |
| `kind` | string | One of the event types above |
| `actor` | string \| null | Identifier of the agent/user that triggered the event |
| `timestamp` | RFC 3339 | Server-side UTC timestamp |
| `payload` | object | Event-specific data (subject to redaction) |

### Reconnect

Use `since=<lastEventId>` to resume from the last received cursor. Events are persisted
in SQLite and survive server restarts. The SSE `id:` field matches `ContextEventV1.id`.

```
GET /v1/events?workspaceId=ws1&channelId=ch1&since=42
```

### Heartbeat

The server sends a keep-alive comment every **15 seconds** to prevent proxy/client timeouts:

```
: keep-alive
```

---

## Metrics

### Endpoint

```
GET /v1/metrics
```

Returns a JSON snapshot of Context OS process-level counters. Requires `Audit` scope on
the team server.

### Response Schema (`MetricsSnapshot`)

```json
{
  "eventsAppended": 1234,
  "eventsBroadcast": 1200,
  "eventsReplayed": 560,
  "sseConnectionsActive": 3,
  "sseConnectionsTotal": 47,
  "sharedSessionsLoaded": 12,
  "sharedSessionsPersisted": 8,
  "activeWorkspaceCount": 2
}
```

| Field | Type | Description |
|-------|------|-------------|
| `eventsAppended` | u64 | Total events written to the SQLite event log |
| `eventsBroadcast` | u64 | Total events pushed to live SSE subscribers |
| `eventsReplayed` | u64 | Total events served via replay (`since` queries) |
| `sseConnectionsActive` | u64 | Currently open SSE connections (opened − closed) |
| `sseConnectionsTotal` | u64 | Lifetime SSE connections opened |
| `sharedSessionsLoaded` | u64 | Shared sessions loaded from disk |
| `sharedSessionsPersisted` | u64 | Shared sessions persisted to disk |
| `activeWorkspaceCount` | usize | Distinct workspace IDs seen since process start |

---

## Redaction

Event payloads delivered via SSE are redacted by default to prevent leaking file contents,
session data, or tool arguments to observers.

### Redaction Levels

| Level | Default | Exposed Fields | Requires |
|-------|---------|---------------|----------|
| `refs_only` | **yes** | `tool`, `kind`, `event_kind`, `workspace_id`, `channel_id`, `id` + `"redacted": true` | — |
| `summary` | no | All metadata preserved; sensitive content fields (`content`, `file_content`, `result`, `output`, `session_data`, `knowledge_value`, `arguments`) replaced with `[redacted]` | — |
| `full` | no | Complete payload, no redaction | `Audit` scope |

### Example: `refs_only` (default)

```json
{
  "tool": "ctx_read",
  "kind": "tool_call_recorded",
  "workspace_id": "ws1",
  "redacted": true
}
```

### Example: `summary`

```json
{
  "tool": "ctx_read",
  "kind": "tool_call_recorded",
  "workspace_id": "ws1",
  "content": "[redacted]",
  "arguments": "[redacted]"
}
```

### Example: `full`

```json
{
  "tool": "ctx_read",
  "kind": "tool_call_recorded",
  "workspace_id": "ws1",
  "content": "use std::sync::Arc;\n...",
  "arguments": { "path": "src/main.rs", "mode": "full" }
}
```

---

## Auth / Scopes (Team Server)

The team server enforces scope-based authorization per bearer token. Tokens are configured
in the team server JSON config with SHA-256 hashes.

### Token Configuration

```json
{
  "tokens": [
    {
      "id": "ci-readonly",
      "sha256Hex": "<lowercase hex of SHA-256(token)>",
      "scopes": ["search", "graph"]
    },
    {
      "id": "admin",
      "sha256Hex": "<lowercase hex of SHA-256(token)>",
      "scopes": ["search", "graph", "artifacts", "index", "events", "sessionMutations", "knowledge", "audit"]
    }
  ]
}
```

### Scopes

| Scope | Grants Access To |
|-------|-----------------|
| `search` | `ctx_read`, `ctx_multi_read`, `ctx_smart_read`, `ctx_search`, `ctx_tree`, `ctx_outline`, `ctx_expand`, `ctx_delta`, `ctx_dedup`, `ctx_prefetch`, `ctx_preload`, `ctx_review`, `ctx_response`, `ctx_task`, `ctx_overview`, `ctx_pack` (+ graph), `ctx_semantic_search` |
| `graph` | `ctx_graph`, `ctx_graph_diagram`, `ctx_impact`, `ctx_callgraph`, `ctx_callers`, `ctx_callees`, `ctx_routes`, `ctx_pack` (+ search) |
| `artifacts` | `ctx_semantic_search` with `artifacts=true` |
| `index` | `ctx_graph` with `action=index-build*`, `ctx_semantic_search` with `action=reindex` |
| `events` | `GET /v1/events` SSE stream |
| `sessionMutations` | Shared session write operations |
| `knowledge` | Knowledge store read/write |
| `audit` | `GET /v1/metrics`, full-payload event access, audit log reads |

### Blocked Tools

The following tools are **never allowed** on the team server (no scope grants access):

- `ctx_shell` / `ctx_execute` — arbitrary command execution
- `ctx_edit` — file modification

### Scope Enforcement

1. **Endpoint-level**: `/v1/events` requires `Events`, `/v1/metrics` requires `Audit`.
2. **Tool-level**: each tool call is mapped to required scopes via `required_scopes()`.
   The request is allowed only if `required_scopes ⊆ token_scopes`.
3. **MCP fallback**: `tools/call` JSON-RPC requests on the MCP transport are also
   scope-checked by parsing the request body in the auth middleware.

### Audit Log

Every tool call and endpoint access is logged to the configured `auditLogPath` as
newline-delimited JSON:

```json
{
  "ts": "2026-05-05T13:00:00+02:00",
  "tokenId": "ci-readonly",
  "workspaceId": "ws1",
  "tool": "ctx_read",
  "method": "/v1/tools/call",
  "allowed": true,
  "deniedReason": null,
  "argumentsMd5": "d41d8cd98f00b204e9800998ecf8427e"
}
```

---

## Server Configuration

### Local Server (`HttpServerConfig`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | string | `127.0.0.1` | Bind address |
| `port` | u16 | `8080` | Bind port |
| `auth_token` | string \| null | none | Bearer token (required for non-loopback) |
| `stateful_mode` | bool | `false` | MCP stateful session mode |
| `max_body_bytes` | usize | `2 MiB` | Max request body size |
| `max_concurrency` | usize | `32` | Max concurrent requests (semaphore) |
| `max_rps` | u32 | `50` | Token-bucket rate limit (requests/sec) |
| `rate_burst` | u32 | `100` | Token-bucket burst capacity |
| `request_timeout_ms` | u64 | `30 000` | Per-request timeout |

### Team Server (`TeamServerConfig`)

Extends the local server with multi-workspace support, token-based auth, and audit logging.
See `rust/src/http_server/team.rs` for the full config schema.

---

## Security

- **Non-loopback binding** requires `--auth-token` (local server) or configured tokens (team server).
- Bearer tokens are compared in **constant time** to prevent timing attacks.
- Team server tokens are stored as **SHA-256 hashes** — raw tokens never touch disk.
- Rate limiting and concurrency guards protect against resource exhaustion.
- Host header validation follows rmcp defaults (loopback-only) unless explicitly overridden.
