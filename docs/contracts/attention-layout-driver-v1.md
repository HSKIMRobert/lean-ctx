# Attention-aware Layout Driver v1 (AttentionLayoutDriverV1)

GitLab: `#2311`

LeanCTX kann Context nicht nur komprimieren, sondern auch **re-layouten**, damit relevante Teile an Positionen landen, die LLMs tatsächlich stärker beachten (“Lost in the Middle” → empirisch eher **L‑Curve** als U‑Curve).

## Ziele

- **Deterministisch**: gleicher Input + gleiche Keywords + gleiche Policy ⇒ gleiche Reihenfolge.
- **Semantic-first**: bei größeren Inhalten zuerst Chunk‑Reordering (Imports/Types/Fns), danach line-level fallback.
- **Policy-gated**: reordering ist **opt-in** pro Profile.
- **Verifier-safe**: Reorder darf keinen Content “verlieren”, nur umsortieren.
- **Bounded**: kleine Inhalte werden nicht re-ordered (Edge Cases).

## Aktivierung (Policy)

Per Profile:

- `profile.layout.enabled = true|false`
- `profile.layout.min_lines = <n>`

Default: `enabled=false`.

## Semantik (v1)

- **Small files**: wenn `lines <= 5` (oder `< min_lines`) ⇒ keine Änderung.
- **Large content**: ab `lines >= 15` wird Chunking versucht:
  1. `detect_chunks(content)`
  2. `order_for_attention(chunks, task_keywords)`
  3. `render_with_bridges(ordered)`
- **Fallback**: sonst line-level scoring + stable tie-break (original index).

## Keywords

Keywords stammen aus dem Task/Intent Kontext (z.B. `task` Argument), extrahiert via `task_relevance::parse_task_hints`.

## Determinism Guarantees

- Sorts haben stabile Tie-breaks (z.B. `start_line`, `original_index`), damit gleiche Scores nicht zu nondeterministic reorder führen.

## Relevanter Code

- Driver: `rust/src/core/attention_layout_driver.rs`
- Chunk reorder: `rust/src/core/semantic_chunks.rs`
- Line-level reorder: `rust/src/core/neural/context_reorder.rs`
- Learned attention curve: `rust/src/core/neural/attention_learned.rs`

