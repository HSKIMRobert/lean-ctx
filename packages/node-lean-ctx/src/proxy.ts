/**
 * Drop-in `compress(messages, { model })` over the local lean-ctx proxy.
 *
 * Posts a chat-style `messages` array to the daemon's deterministic
 * `POST /v1/compress` endpoint and returns the rewritten messages. Only text
 * payloads are compressed; images, tool-call blocks and ids pass through
 * untouched, and the output is byte-stable for provider prompt caching.
 */

import { resolveBaseUrl, resolveToken } from "./discovery";
import { LeanCtxAuthError, LeanCtxConnectionError, LeanCtxError } from "./errors";

export type Message = Record<string, unknown>;

export interface CompressStats {
  original_tokens: number;
  compressed_tokens: number;
  saved_tokens: number;
  saved_pct: number;
  tokenizer?: string;
  model?: string | null;
}

export interface CompressResult {
  messages: Message[];
  stats: CompressStats;
}

export interface ProxyClientOptions {
  baseUrl?: string;
  token?: string;
  timeoutMs?: number;
}

export interface CompressOptions extends ProxyClientOptions {
  model?: string;
}

const DEFAULT_TIMEOUT_MS = 30000;

/** Reusable client for the local lean-ctx proxy `/v1/compress` endpoint. */
export class ProxyClient {
  readonly baseUrl: string;
  private readonly token?: string;
  private readonly timeoutMs: number;

  constructor(options: ProxyClientOptions = {}) {
    this.baseUrl = resolveBaseUrl(options.baseUrl);
    this.token = resolveToken(options.token);
    this.timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  }

  /** Compress `messages` and return the rewritten list plus savings stats. */
  async compress(messages: Message[], model?: string): Promise<CompressResult> {
    if (!Array.isArray(messages)) {
      throw new TypeError("messages must be an array of chat-message objects");
    }
    const payload: Record<string, unknown> = { messages };
    if (model) payload.model = model;

    const data = await this.post("/v1/compress", payload);
    const out = (data as { messages?: unknown }).messages;
    if (!Array.isArray(out)) {
      throw new LeanCtxError("malformed /v1/compress response: 'messages' missing");
    }
    const stats = (data as { stats?: unknown }).stats;
    return {
      messages: out as Message[],
      stats: (typeof stats === "object" && stats !== null ? stats : {}) as CompressStats,
    };
  }

  private async post(path: string, payload: Record<string, unknown>): Promise<unknown> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = { "Content-Type": "application/json" };
    if (this.token) headers.Authorization = `Bearer ${this.token}`;

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);

    let response: Response;
    try {
      response = await fetch(url, {
        method: "POST",
        headers,
        body: JSON.stringify(payload),
        signal: controller.signal,
      });
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      throw new LeanCtxConnectionError(
        `could not reach the lean-ctx proxy at ${this.baseUrl} (${reason}). ` +
          "Is the daemon running? Try: lean-ctx proxy enable",
      );
    } finally {
      clearTimeout(timer);
    }

    if (response.status === 401 || response.status === 403) {
      throw new LeanCtxAuthError(
        `proxy rejected the request (HTTP ${response.status}). ` +
          "Set LEAN_CTX_PROXY_TOKEN or pass { token }.",
      );
    }
    if (!response.ok) {
      const detail = (await response.text()).trim();
      throw new LeanCtxError(`POST ${path} failed (HTTP ${response.status}): ${detail}`);
    }
    try {
      return await response.json();
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      throw new LeanCtxError(`invalid JSON response from ${url}: ${reason}`);
    }
  }
}

/**
 * Compress a chat `messages` array, returning the rewritten messages.
 *
 * ```ts
 * import { compress } from "lean-ctx";
 * const messages = await compress(history, { model: "claude-sonnet-4" });
 * ```
 *
 * For token-savings stats, use {@link ProxyClient} directly.
 */
export async function compress(
  messages: Message[],
  options: CompressOptions = {},
): Promise<Message[]> {
  const { model, ...clientOptions } = options;
  const client = new ProxyClient(clientOptions);
  const result = await client.compress(messages, model);
  return result.messages;
}
