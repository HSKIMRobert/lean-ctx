/** JSON object accepted by the OCLA envelope endpoint. */
export type JsonObject = Record<string, unknown>;
export type OclaApiVersion = "ocla/v1";
export interface HealthResponse {
  status: "ok";
  version: OclaApiVersion | string;
}
export type OclaCapabilityKind =
  | "observation_hook"
  | "usage_sink"
  | "metrics_exporter"
  | "savings_ledger"
  | "intent_classifier"
  | "outcome_tracker"
  | "compression_provider"
  | "response_optimizer"
  | "model_router"
  | "efficiency_analyzer"
  | "config_tuner"
  | "experiment_runner"
  | "connector_scheduler"
  | "agent_gateway";
export type OclaCapabilityStatus = "available" | "degraded" | "unavailable";
export interface OclaCapability {
  kind: OclaCapabilityKind;
  api_version: OclaApiVersion | string;
  status: OclaCapabilityStatus;
  limits: Record<string, number>;
}
export interface CapabilitiesResponse {
  version: OclaApiVersion | string;
  capabilities: OclaCapability[];
}
export interface OclaRequestContext {
  request_id: string;
  session_id: string;
  agent_id: string;
  content_ref: string;
  tenant_id: string | null;
}
export type TokenEnvelopeSurface = "mcp" | "proxy" | "shell" | "agent";
export type TokenFlowDirection = "input" | "output";
export interface TokenBalanceV1 {
  original_tokens: number;
  materialized_tokens: number;
  delivered_tokens: number;
  provider_billed_tokens: number;
}
export interface CanonicalTokenEnvelopeV1 {
  schema_version: 1;
  context: OclaRequestContext;
  surface: TokenEnvelopeSurface;
  direction: TokenFlowDirection;
  provider: string;
  model: string;
  token_balance: TokenBalanceV1;
  route_ref: string | null;
  policy_ref: string | null;
  idempotency_key: string;
}
export interface AgentEnvelopeV1 {
  schema_version: 1;
  relay_id: string;
  context: OclaRequestContext;
  from_agent_id: string;
  to_agent_id: string;
  capsule_ref: string;
  budget_tokens: number;
}
export type EnvelopeResponse = CanonicalTokenEnvelopeV1;
export interface LedgerSummary {
  events: number;
  tokens: number;
  usd: number;
}
export interface OclaErrorResponse {
  error: string;
}
