# Benchmarks

The benchmark suite is reproducible and local-only. It does not require model
API keys, telemetry, accounts, or remote services.

Run:

```bash
npm run benchmarks
```

Current fixtures cover:

- Shell output compression.
- Repo context pack savings versus broad scans.
- Document-conversion handoff cleanup.

Reported metrics:

- Original token estimate.
- Optimized token estimate.
- Saved tokens.
- Percent saved.
- Quality check label.

LLM-judged quality benchmarks are intentionally not part of the default run.
When added, they must stay optional and clearly labelled because they can vary
by provider, model, prompt, and date.
