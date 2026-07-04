# Optimization Reporting CLI

Generate a metrics-only optimization report:

```bash
node scripts/optimization-report.mjs --input safe-optimization-snapshot.json
node scripts/optimization-report.mjs --input safe-optimization-snapshot.json --json
```

Snapshot fixture shape:

```json
{
  "schemaVersion": 1,
  "source": "fixture:safe-token-metrics",
  "tokenXray": {
    "promptTokens": 1200,
    "completionTokens": 300,
    "toolTokens": 500,
    "contextWindow": 8000,
    "segments": [{ "label": "repo map", "tokens": 700 }]
  },
  "redundancy": {
    "duplicateBlocks": 2,
    "duplicateTokens": 250,
    "repeatedToolCalls": 1
  },
  "cacheEfficiency": {
    "cacheReadTokens": 900,
    "cacheWriteTokens": 100,
    "totalInputTokens": 2000
  },
  "compaction": {
    "beforeTokens": 6000,
    "afterTokens": 2200,
    "triggerTokens": 7000,
    "summaryTokens": 650
  },
  "modelRouting": {
    "fallbackCount": 1,
    "routes": [
      {
        "model": "gpt-5-codex",
        "requests": 4,
        "inputTokens": 1200,
        "outputTokens": 300,
        "reason": "code edits"
      }
    ]
  },
  "rtkPresets": {
    "mode": "auto",
    "command": "npm test",
    "files": ["src/app.test.ts"],
    "output": "FAIL src/app.test.ts\nAssertionError: expected safe metric\n"
  }
}
```

The CLI rejects raw prompt-bearing keys such as `prompt`, `messages`, `content`, `text`, `raw`, and `transcript`.
