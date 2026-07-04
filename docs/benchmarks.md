# Benchmarks

The benchmark suite is reproducible and local-only. It does not require model
API keys, telemetry, accounts, or remote services.

Run:

```bash
npm run benchmarks
```

Fixtures live in `benchmarks/fixtures.json` and cover:

- Shell output compression.
  - Noisy test logs.
  - Stack-trace summaries.
- Repo context pack savings versus broad scans.
- Document-conversion handoff cleanup.

Reported metrics:

- Original token estimate.
- Optimized token estimate.
- Saved tokens.
- Percent saved.
- Latency overhead in milliseconds.
- Relevant fact retention.
- Wrong omission rate.
- Static agent success proxy where applicable.
- Quality check label.

LLM-judged quality benchmarks are intentionally not part of the default run.
When added, they must stay optional and clearly labelled because they can vary
by provider, model, prompt, and date.
