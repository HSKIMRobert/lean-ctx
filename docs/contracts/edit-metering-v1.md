# Edit Metering v1 — Anchored vs str_replace Efficiency Channel

Status: shipped (GL #1008, phase 4)
Owner: core engine
Consumers: `ctx_metrics`, dashboard (`/api/stats` → `edit_efficiency`), A/B eval harness

## Problem

Anchored editing's pitch is quantitative: referencing the preimage by
`(line, hash)` instead of reproducing it as `old_string` saves *output* tokens
(the expensive kind, ~5× input) and removes retry round-trips. Without a
measurement channel that claim is marketing. Per the honest-metering
philosophy (#361), every number must be measured per edit — never estimated,
never extrapolated.

## Signals

Recorded by the two edit paths themselves, in-process:

| Signal | Recorded by | Semantics |
|---|---|---|
| `anchored_calls` | `ctx_patch` success path | Successful patch calls (a batch counts once) |
| `anchored_ops` | `ctx_patch` success path | Anchored ops applied across those calls |
| `anchored_avoided_output_tokens` | `ctx_patch` success path | Σ per op: `max(0, tokens(replaced span) − tokens(anchor args))` |
| `anchored_conflicts` | `ctx_patch` CONFLICT path | Stale-anchor responses (each = one self-heal round-trip) |
| `str_replace_calls` | `ctx_edit` success path | Successful str_replace edits |
| `str_replace_old_string_tokens` | `ctx_edit` success path | Σ `tokens(old_string)` actually paid in output |
| `str_replace_misses` | `ctx_edit` miss path | `old_string`-not-found responses (blind retry round-trips) |

## Honesty rules

- **Preimage math, measured per applied op** — the replaced span is tokenized
  *before* the splice; the anchor-args cost is subtracted. What a str_replace
  of the same span would have paid is exactly the span text; no multiplier.
- **Floored at 0 per op** — a one-liner can be cheaper to quote than its
  `line:hash` anchor; that op books 0 avoided tokens, never negative and never
  hides in an average. The A/B benchmark asserts this stays a tiny-span
  exception (`rust/tests/edit_reliability.rs`).
- **`op=create` books 0** — both paths emit the full new content; nothing is
  avoided, so nothing is claimed.
- **Separate channel** — values are never folded into the read-gain ledger
  (no double counting) and never printed in tool output bodies (#498
  determinism).

## Persistence

`~/.lean-ctx/edit_metering.json` (respects `LEAN_CTX_DATA_DIR`), atomic
tmp+rename writes, flushed every 5 recordings and on shutdown via
`tool_lifecycle::flush_all`. Loaded once per process; missing/partial files
deserialize field-by-field (`serde(default)`), so adding fields is
backward-compatible.

## Observability

- `ctx_metrics` → `Edit efficiency (anchored vs str_replace, all-time)`
  section; hidden until either edit path has been used.
- Dashboard ROI view → **Edit Efficiency** card (`/api/stats` →
  `edit_efficiency`), labelled **measured**.
- A/B benchmark: `cargo test --test edit_reliability -- --nocapture` prints
  success rates and argument-token costs for identical fixes across 5
  languages (see Journey 11 §9).

## Invariants

- Recording never blocks an edit: store access is lock-guarded; failure to
  lock skips the record.
- Counters are saturating (`u64`), monotone, all-time; there is no windowing
  in v1.
- `anchored_conflicts` counts responses, not ops: one CONFLICT reply = one
  round-trip regardless of how many ops went stale.
