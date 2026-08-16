# Benchmarks

The benchmark suite is reproducible and local-only. It does not require model
API keys, telemetry, accounts, or remote services.

Run:

```bash
npm run benchmarks
```

The default run writes two ignored local artifacts:

- `benchmarks/results/manifest.json` for CI and other tooling;
- `benchmarks/results/summary.md` for human review.

It also prints the same manifest JSON to stdout for existing automation. Use
`node scripts/run-benchmarks.mjs --check` to fail only when a metric exceeds an
explicit threshold in `benchmarks/schema.json` compared with the stored
`benchmarks/baseline.json`. Use `--output-dir <path>` to retain artifacts in a
CI-specific directory, or `--no-write` when only stdout is needed.

The offline profile never invokes Headroom, RTK, an inference runtime, or a
network service. Its manifest therefore records `not-invoked-offline` for the
two tool versions unless CI supplies
`SWITCHBOARD_BENCHMARK_HEADROOM_VERSION` and
`SWITCHBOARD_BENCHMARK_RTK_VERSION`. The fixture version, Git commit, platform,
and profile make each result attributable without making the fixture metrics
non-deterministic.

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

## Live harness contract

The Rust `LiveBenchmarkTarget` contract is a separate developer-facing layer.
It defines the four required variants (B00, B10, B01, B11) without enabling
automatic routing or contacting a provider. Its mock endpoint test proves that
all four combinations can be warmed and run locally before runtime-specific
adapters such as vLLM are introduced.
