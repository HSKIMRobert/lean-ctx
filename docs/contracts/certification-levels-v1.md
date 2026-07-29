# OCLA Certification Levels — v1

## Overview

Three levels of conformance for implementations of the OCLA wire contract.
Each level builds on the previous one.

## Level 1: Schema Valid

**Requirement:** Implementation can produce and consume JSON documents that
validate against `ocla-wire-v1.schema.json`.

**Evidence:**

- JSON Schema validation passes for all required document types
- Deserialization of all golden fixtures succeeds
- Unknown fields are preserved (no `deny_unknown_fields`)

**Automated check:** `lean-ctx conformance --level 1`

## Level 2: Roundtrip Deterministic

**Requirement:** Level 1 + serialize→deserialize roundtrip produces
byte-identical output (modulo field ordering).

**Evidence:**

- All golden fixtures roundtrip without data loss
- Payload variants (Messages, StreamChunk, ToolCall, Usage) roundtrip
- Token accounting invariants hold (prompt + completion = total)

**Automated check:** `lean-ctx conformance --level 2`

## Level 3: Conformance Pass

**Requirement:** Level 2 + passes the full verifier conformance suite
(`ocla-verifier-conformance-v1.md`), including:

- All 18 required cases
- Adversarial rejection cases
- Size limits (64 KiB)
- Timeout limits (5s per case)

**Evidence:**

- Verifier scorecard with `all_passed: true`
- Must be reproducible on clean CI environment

**Automated check:** `scripts/verify-ocla-contract-suite.py --verifier BINARY`

## Current Status

| Implementation | Level | Notes |
|---|---|---|
| lean-ctx engine | 3 | Reference implementation |
| lean-ctx-client (Rust) | 3 | External consumer, CI-verified |
| TypeScript SDK | 1 | Schema types synced in E17 |
| Python SDK | 1 | Pydantic models synced in E17 |
| Go SDK | 1 | Types synced in E17 |
