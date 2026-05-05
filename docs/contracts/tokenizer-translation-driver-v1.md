# Tokenizer-aware Translation Driver v1 (TokenizerTranslationDriverV1)

GitLab: `#2310`

LeanCTX nutzt “Translation” als letzten Optimierungsschritt für **synthetische Context-Formate** (z.B. TDD Signatures), damit Output-Tokens modell-/tokenizer-spezifisch reduziert werden können.

## Ziele

- **Deterministisch**: gleicher Input + gleicher `model_key` + gleiche Profile Policy ⇒ gleiches Translation Ruleset.
- **Safe defaults**: Default verändert bestehende CRP/TDD Formate **nicht** (opt-in).
- **Tokenizer-aware**: Rulesets können Unicode → ASCII tauschen, wenn Unicode im Ziel-Tokenizer teurer ist.
- **Verifier-safe**: Translation darf keine File Paths oder Identifiers “kaputtoptimieren”.
- **Bounded**: Translation läuft auf Tool-Outputs, aber wird für JSON-Ausgaben übersprungen (machine-readable bleibt exakt).

## Aktivierung (Policy)

Translation wird über Profile gesteuert:

- `profile.translation.enabled = true|false`
- `profile.translation.ruleset = "legacy" | "ascii" | "auto"`

Default: `enabled=false`, `ruleset="legacy"`.

## Deterministische Ruleset Selection

`ruleset="auto"` verwendet den **Model Key** aus:

- `LEAN_CTX_MODEL` (oder `LCTX_MODEL`)

Heuristik (v1):

- OpenAI/GPT-family (`model` enthält `gpt`) → `ascii`
- sonst → `legacy`

## Überspringen von JSON Outputs

Tool-Outputs, die als JSON parsebar sind (`{...}` oder `[...]`), werden nicht verändert.

## Messung / Bench

`ctx_benchmark` zeigt token_cost (o200k_base) vor/nach Ruleset-Translation für TDD/Signature Outputs.

## Relevanter Code

- Driver: `rust/src/core/tokenizer_translation_driver.rs`
- Token cost oracle: `rust/src/core/tokens.rs`
- Empirische safe rules: `rust/src/core/neural/token_optimizer.rs`
- Verification safety: `rust/src/core/output_verification.rs`

