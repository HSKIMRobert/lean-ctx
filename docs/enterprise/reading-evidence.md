# Reading LeanCTX Evidence — A Guide for Auditors

**Audience:** auditors, compliance officers and assessors with **no
LeanCTX knowledge and no command-line experience beyond running one
program**. Technical background is helpful but not required.

**What you receive:** one file, e.g. `evidence-bundle_2026-05_2026-06.zip`,
and one small program, `leanctx-verify` (Windows/macOS/Linux). Nothing to
install, no network connection needed, no access to the audited
organisation's systems.

---

## 1. What LeanCTX is, in one paragraph

LeanCTX sits between an organisation's AI coding agents and their data.
Every file an AI reads, every command it runs, flows through LeanCTX,
which enforces rules (what may be read, what must be redacted, how much
context an AI may consume) and records what happened. The evidence bundle
is the export of those records for a chosen period, packaged so that you
can check its integrity yourself.

## 2. What the bundle contains

| File in the ZIP | What it is | What it answers |
|---|---|---|
| `manifest.json` | Signed table of contents | Is this bundle complete and untampered? |
| `audit/trail.jsonl` | The activity log, one line per AI action | What did AI agents actually do? |
| `policies/*.resolved.json` | The rules that were in force | What was allowed, forbidden, redacted? |
| `coverage/cgb.json` | Self-assessment against LeanCTX's public governance benchmark | Which governance controls were active? |
| `coverage/<framework>.json` | Mapping to EU AI Act / ISO 42001 / SOC 2 | Which framework controls were technically enforced? |

## 3. How to verify — three minutes

1. Put `bundle.zip` and `leanctx-verify` in the same folder.
2. Open a terminal in that folder and run:

```
leanctx-verify bundle.zip
```

3. Read the verdict. Every line is one independent check:

```
  [PASS] archive + manifest        evidence-bundle v1, 5 archive entries
  [PASS] file inventory + SHA-256  4 files match their manifest hashes
  [PASS] audit chain replay        626 entries replay from anchor to head
  [PASS] manifest signature        Ed25519 valid (out-of-band key)
  [PASS] entry signatures          626 entries signed and verified

result: VALID
```

`result: VALID` means: **no byte of this bundle changed since it was
generated and signed.** Any modification — a deleted log line, an edited
number, one flipped bit — turns at least one PASS into FAIL.

**Strongly recommended:** ask the organisation for their *public key*
through a separate channel (e-mail from a known contact, their security
page) and run `leanctx-verify bundle.zip --pubkey <the key>`. See §6.

## 4. What each check proves

**File inventory + SHA-256.** Every file is "fingerprinted" (SHA-256).
The manifest lists the expected fingerprints; the tool recomputes them
from the actual files. A single changed character changes the
fingerprint completely. This proves *the files are exactly the ones the
manifest describes*.

**Audit chain replay.** Each log entry contains the fingerprint of the
*previous* entry — like numbered, interlocking wax seals. The tool
recomputes every seal from the entry's own content. This proves *no
entry was modified, reordered, inserted or deleted inside the period* —
removing even one line breaks every seal after it.

**Manifest signature.** The manifest is signed with the organisation's
private key (Ed25519, the same cryptography used in passports and SSH).
Only the holder of the private key can produce the signature; anyone can
check it. This proves *who* attests this bundle.

**Entry signatures.** Each log line additionally carries its own
signature, binding each individual action to the same key.

## 5. What this evidence does NOT prove — read this

We state the limits explicitly; an evidence format that hides its limits
is not evidence.

1. **Events are protected from the moment they are recorded — not
   before.** If an attacker fully controlled the machine *while events
   were happening*, they could have prevented recording. The chain
   proves history wasn't *rewritten afterwards*; it cannot prove events
   were never suppressed at the source. (Mitigation: entries are written
   and sealed immediately, not batched.)
2. **A bundle covers its period, relative to its anchor.** The first
   entry references the seal of the last entry *before* the period
   (`anchor_prev_hash`). Continuity across bundles holds when the
   previous bundle's final seal matches the next bundle's anchor — check
   this when you receive several periods.
3. **The embedded key is self-attested.** Without an out-of-band key
   (§6), the bundle proves internal consistency, but anyone could have
   generated and signed it. With the out-of-band key it proves origin.
4. **Coverage reports are machine statements, not legal opinions.** They
   say "this rule was technically enforced and here is the test that
   proves the mechanism works". Whether that satisfies a legal
   requirement is an assessment for humans — LeanCTX's mapping documents
   the residual gaps honestly (`coverage = none` rows with reasons).
5. **LeanCTX governs the context pipeline.** It does not govern what
   happens outside it (training data, the host operating system, what a
   human does with an AI's answer).

## 6. Verifying the signer (out-of-band key)

Ask your contact for the organisation's **Ed25519 public key** (64 hex
characters) via a channel you already trust. Then:

```
leanctx-verify bundle.zip --pubkey 3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29
```

If the manifest was signed by a different key, the signature check
fails. Keep the key on file — every future bundle from this
organisation should verify against the same key (a key change should
come with an explanation, like a certificate rotation).

## 7. Reading the audit log itself

`audit/trail.jsonl` is plain text — every line one action:

```json
{"timestamp":"2026-06-11T08:47:53Z","agent_id":"mcp-22723","tool":"ctx_read",
 "role":"developer","event_type":"tool_call","output_tokens":974, …}
```

Useful fields for sampling: `timestamp` (when), `agent_id` (which AI
agent), `tool` (what kind of action), `role` (under which permission
profile), `event_type` — `tool_call` is normal activity; `tool_denied`,
`path_jail_violation`, `budget_exceeded`, `security_violation`,
`secret_detected` are enforcement events you may want to inspect: they
show the rules *firing*, with the same tamper-evidence as everything
else.

`input_hash` is a fingerprint of the action's parameters, not the
parameters themselves — file contents never leave the organisation in
this bundle (deliberately: the log proves *that and what kind of*
activity happened without exporting the data the rules protect).

## 8. Checklist

- [ ] `leanctx-verify bundle.zip` → `result: VALID`
- [ ] Verified with out-of-band `--pubkey` (§6)
- [ ] Period in `manifest.json` matches the audit scope
- [ ] For multi-period audits: anchors chain across bundles (§5.2)
- [ ] Sampled enforcement events (`tool_denied`, …) match the
      organisation's stated policy
- [ ] Framework coverage rows marked GAP discussed with the organisation
- [ ] Re-run on a second machine if first run was on theirs

## 9. Glossary

| Term | Meaning |
|---|---|
| SHA-256 | One-way fingerprint; any change to the input changes it |
| Hash chain | Each record sealed with the previous record's fingerprint |
| Ed25519 | Digital-signature scheme; private key signs, public key verifies |
| Manifest | Signed table of contents of the bundle |
| Anchor | The seal linking this period to the history before it |
| Policy pack | The machine-enforced rule set in force during the period |

*Contract: `docs/contracts/evidence-bundle-v1.md` · Verifier source:
`packages/leanctx-verify/` (Apache-2.0, ~600 lines, independently
implementable from the contract alone).*
