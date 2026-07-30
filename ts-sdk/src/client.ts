import type {
  CapabilitiesResponse,
  CanonicalTokenEnvelopeV1,
  EnvelopeResponse,
  HealthResponse,
  JsonObject,
  LedgerSummary,
} from "./types.js";
import { streamEvents } from "./streaming.js";

export type CapsuleData = { capsule_ref: string; data: string };

function stripTrailingSlashes(value: string): string {
  let end = value.length;
  while (end > 0 && value.charCodeAt(end - 1) === 47) end--;
  return value.slice(0, end);
}

export class OclaClient {
  readonly baseUrl: string;
  readonly apiKey: string;

  constructor(baseUrl: string, apiKey = "") {
    const normalized = stripTrailingSlashes(baseUrl.trim());
    if (!normalized) {
      throw new Error("OclaClient: baseUrl is required");
    }
    this.baseUrl = normalized;
    this.apiKey = apiKey;
  }

  async health(): Promise<HealthResponse> {
    return this.get<HealthResponse>("/ocla/v1/health");
  }
  async capabilities(): Promise<CapabilitiesResponse> {
    return this.get<CapabilitiesResponse>("/ocla/v1/capabilities");
  }
  async validateEnvelope(envelope: object): Promise<EnvelopeResponse> {
    return this.request<EnvelopeResponse>("/ocla/v1/envelope", {
      method: "POST",
      body: JSON.stringify(envelope),
    });
  }
  async registerCapsule(data: string): Promise<string> {
    const response = await this.request<{ capsule_ref: string }>(
      "/ocla/v1/capsule",
      {
        method: "POST",
        body: data,
        headers: { "Content-Type": "text/plain" },
      },
    );
    return response.capsule_ref;
  }
  async resolveCapsule(capsuleRef: string): Promise<CapsuleData> {
    return this.get<CapsuleData>(`/ocla/v1/capsule/${capsuleRef}`);
  }
  async forkCapsule(capsuleRef: string, budgetTokens: number): Promise<string> {
    const response = await this.request<{ capsule_ref: string }>(
      `/ocla/v1/capsule/${capsuleRef}/fork`,
      {
        method: "POST",
        body: JSON.stringify({ budget_tokens: budgetTokens }),
      },
    );
    return response.capsule_ref;
  }
  async ledgerSummary(): Promise<LedgerSummary> {
    return this.get<LedgerSummary>("/ocla/v1/ledger/summary");
  }
  async *streamEnvelopes(): AsyncGenerator<CanonicalTokenEnvelopeV1> {
    const resp = await fetch(`${this.baseUrl}/v1/events`, {
      headers: {
        Accept: "text/event-stream",
        Authorization: `Bearer ${this.apiKey}`,
      },
    });
    if (!resp.ok) {
      const detail = await resp.text();
      const suffix = detail.trim() ? `: ${detail.trim()}` : "";
      throw new Error(`OCLA stream failed (${resp.status})${suffix}`);
    }
    for await (const event of streamEvents(resp)) {
      if (event.type === "envelope") {
        yield event.data as CanonicalTokenEnvelopeV1;
      }
    }
  }
  private async get<T>(path: string): Promise<T> {
    return this.request<T>(path, { method: "GET" });
  }
  private async request<T>(path: string, init: RequestInit): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      ...init,
      headers: {
        Accept: "application/json",
        ...(init.body === undefined ? {} : { "Content-Type": "application/json" }),
        ...(this.apiKey ? { Authorization: `Bearer ${this.apiKey}` } : {}),
        ...init.headers,
      },
    });

    if (!response.ok) {
      const detail = await response.text();
      const suffix = detail.trim() ? `: ${detail.trim()}` : "";
      throw new Error(`OCLA request failed (${response.status})${suffix}`);
    }

    return (await response.json()) as T;
  }
}
export type { JsonObject };
