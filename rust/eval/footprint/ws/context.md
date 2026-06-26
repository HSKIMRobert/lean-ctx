# Footprint eval workspace

Placeholder corpus for the footprint ablation suite (`eval/footprint-suite.ndjson`).

The signal under test is lean-ctx's own injected footprint (rules block, tool
schemas, wakeup briefing) — not this directory. The QA scorer ignores the
workspace for these tasks; the directory exists only so relative workspace paths
resolve uniformly across the harness.
