"""Async HTTP client for the OCLA Wire API."""

from __future__ import annotations

from types import TracebackType
from typing import Any, Optional

import httpx

from .models import (
    CapabilitiesResponse,
    EnvelopeResponse,
    HealthResponse,
    LedgerSummary,
)


class OclaClient:
    """Call an OCLA Wire API endpoint over async HTTP."""

    def __init__(
        self,
        base_url: str,
        *,
        client: Optional[httpx.AsyncClient] = None,
        timeout: float = 10.0,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._client = client or httpx.AsyncClient(timeout=timeout)
        self._owns_client = client is None

    async def __aenter__(self) -> OclaClient:
        return self

    async def __aexit__(
        self,
        exc_type: Optional[type[BaseException]],
        exc_value: Optional[BaseException],
        traceback: TracebackType | None,
    ) -> None:
        await self.aclose()

    async def aclose(self) -> None:
        """Close the underlying HTTP client when this instance owns it."""

        if self._owns_client:
            await self._client.aclose()

    async def health(self) -> HealthResponse:
        """Return OCLA service health."""

        response = await self._client.get(self._url("health"))
        response.raise_for_status()
        return HealthResponse.model_validate(response.json())

    async def capabilities(self) -> CapabilitiesResponse:
        """Return the capabilities registered by the OCLA service."""

        response = await self._client.get(self._url("capabilities"))
        response.raise_for_status()
        return CapabilitiesResponse.model_validate(response.json())

    async def validate_envelope(self, envelope: dict[str, Any]) -> EnvelopeResponse:
        """Validate and return a canonical token envelope."""

        response = await self._client.post(self._url("envelope"), json=envelope)
        response.raise_for_status()
        return EnvelopeResponse.model_validate(response.json())

    async def ledger_summary(self) -> LedgerSummary:
        """Return the current compact savings-ledger summary."""

        response = await self._client.get(self._url("ledger/summary"))
        response.raise_for_status()
        return LedgerSummary.model_validate(response.json())

    def _url(self, route: str) -> str:
        return f"{self._base_url}/ocla/v1/{route}"
