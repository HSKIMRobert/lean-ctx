# Coverage Classes — v1

Coverage classes categorize the types of AI coding tasks that lean-ctx
compresses. A Shadow Pilot must validate at least 2 classes to confirm
real-world effectiveness.

## Classes

| Class | Description | Typical Savings | Quality Risk |
|---|---|---|---|
| `rust` | Rust source code, Cargo configs, test files | 70-85% | Low |
| `typescript` | TS/JS source, configs, test files | 65-80% | Low |
| `python` | Python source, configs, notebooks | 65-80% | Low |
| `markdown` | Documentation, READMEs, guides | 80-95% | Very Low |
| `shell` | Shell output, command results, logs | 85-95% | Very Low |
| `config` | JSON, YAML, TOML configuration files | 60-75% | Medium |
| `mixed` | Multi-language contexts, cross-file reads | 55-70% | Medium |
| `large-file` | Files >10KB, generated code, data files | 75-90% | Low |

## Minimum Pilot Coverage

A valid Shadow Pilot must include:

- At least 2 different source code classes (e.g., `rust` + `typescript`)
- At least 100 requests per class
- At least 1 week of data per class

## Measurement

Coverage classes are automatically detected by lean-ctx based on:

1. File extension of the read target
2. Tree-sitter language detection
3. Shell command pattern matching

Report via: `lean-ctx gain --by-class --json`

## ETPAO Mapping

| ETPAO Dimension | Measurement |
|---|---|
| **E**fficiency | Token savings % per class |
| **T**hroughput | Requests/minute per class |
| **P**recision | Quality score distribution |
| **A**ccuracy | Roundtrip determinism |
| **O**utput | Compression ratio stability |
