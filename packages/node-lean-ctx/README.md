# lean-ctx (Node SDK)

Context compression for AI agents — a thin, dependency-free client for the local
[lean-ctx](https://leanctx.com) daemon.

```bash
npm install lean-ctx
```

## Drop-in `compress(messages, { model })`

Compress a chat-style `messages` array before sending it to any model. Only text
payloads are rewritten through lean-ctx's deterministic funnel; images,
tool-call blocks and ids pass through untouched, and the output is byte-stable so
it stays friendly to provider prompt caching.

```ts
import { compress } from "lean-ctx";

let messages = [
  { role: "system", content: "You are a helpful assistant." },
  { role: "user", content: largeLogOrFileDump },
];

messages = await compress(messages, { model: "claude-sonnet-4" });
// → send `messages` to your provider as usual
```

Works with both OpenAI-style (`content: "string"`) and Anthropic-style
(`content: [{ type: "text", … }, { type: "tool_result", … }]`) messages.

### Token-savings stats

```ts
import { ProxyClient } from "lean-ctx";

const result = await new ProxyClient().compress(messages, "gpt-4o");
console.log(result.stats.saved_tokens, result.stats.saved_pct);
messages = result.messages;
```

## Configuration

The endpoint and session token are auto-discovered from the running daemon. Every
step is overridable:

| Setting | Env var | Default |
| --- | --- | --- |
| Proxy URL | `LEAN_CTX_PROXY_URL` | `http://127.0.0.1:<port>` |
| Proxy port | `LEAN_CTX_PROXY_PORT` | `config.toml` `proxy_port`, else UID-derived |
| Session token | `LEAN_CTX_PROXY_TOKEN` | `<data_dir>/session_token` |

Or pass them explicitly (useful in CI / against a remote proxy):

```ts
await compress(messages, { baseUrl: "http://127.0.0.1:4444", token: "…" });
```

If the daemon is not running, `compress()` rejects with `LeanCtxConnectionError`;
an unauthenticated request rejects with `LeanCtxAuthError`. Both extend
`LeanCtxError`.

## Other helpers

`LeanCtxClient` wraps the `lean-ctx` binary for `read` / `search` / `shell` /
`gain` / `benchmark`, and `createLeanCtxTool` exposes a Vercel AI SDK tool.

## License

MIT
