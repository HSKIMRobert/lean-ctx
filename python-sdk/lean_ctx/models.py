"""Pydantic models for OCLA Wire API responses."""

from __future__ import annotations

from typing import Literal, Optional

from pydantic import BaseModel, ConfigDict, Field


OCLA_API_VERSION = "ocla/v1"
U64_MAX = 2**64 - 1


class WireModel(BaseModel):
    """Base model that rejects fields outside the public wire contract."""

    model_config = ConfigDict(extra="forbid")


class HealthResponse(WireModel):
    """Response from ``GET /ocla/v1/health``."""

    status: Literal["ok"]
    version: Literal["ocla/v1"]


class ErrorResponse(WireModel):
    """Error body returned when an OCLA request is rejected."""

    error: str
    code: Optional[str] = None


class OclaRequestContext(WireModel):
    """Lineage identifiers carried by a token envelope."""

    request_id: str
    session_id: str
    agent_id: str
    content_ref: str
    tenant_id: Optional[str]


class TokenBalance(WireModel):
    """Token counts recorded at each OCLA lifecycle stage."""

    original_tokens: int = Field(ge=0, le=U64_MAX)
    materialized_tokens: int = Field(ge=0, le=U64_MAX)
    delivered_tokens: int = Field(ge=0, le=U64_MAX)
    provider_billed_tokens: int = Field(ge=0, le=U64_MAX)


class EnvelopeResponse(WireModel):
    """Validated canonical token envelope returned by the envelope endpoint."""

    schema_version: Literal[1]
    context: OclaRequestContext
    surface: Literal["mcp", "proxy", "shell", "agent"]
    direction: Literal["input", "output"]
    provider: str
    model: str
    token_balance: TokenBalance
    route_ref: Optional[str] = None
    policy_ref: Optional[str] = None
    idempotency_key: str


class Capability(WireModel):
    """One registered OCLA capability."""

    kind: Literal[
        "observation_hook",
        "usage_sink",
        "metrics_exporter",
        "savings_ledger",
        "intent_classifier",
        "outcome_tracker",
        "compression_provider",
        "response_optimizer",
        "model_router",
        "efficiency_analyzer",
        "config_tuner",
        "experiment_runner",
        "connector_scheduler",
        "agent_gateway",
    ]
    api_version: Literal["ocla/v1"]
    status: Literal["available", "degraded", "unavailable"]
    limits: dict[str, int]


class CapabilitiesResponse(WireModel):
    """Response from ``GET /ocla/v1/capabilities``."""

    version: Literal["ocla/v1"]
    capabilities: list[Capability]


class LedgerSummary(WireModel):
    """Response from ``GET /ocla/v1/ledger/summary``."""

    events: int = Field(ge=0)
    tokens: int = Field(ge=0, le=U64_MAX)
    usd: float
