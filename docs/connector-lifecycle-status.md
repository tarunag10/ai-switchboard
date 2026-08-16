# Connector lifecycle status matrix

> Generated from `connectors/lifecycle-fixtures.json` by `node scripts/check-connector-lifecycle-matrix.mjs --check`. Do not label a connector **Managed** unless every lifecycle stage has named fixture-test proof.

| Connector | Detect | Preview | Backup | Apply | Verify | Rollback | Off | Fixture proof | UI status |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| Claude Code | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| Codex | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| Gemini CLI | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| OpenCode | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| Cursor | ✓ | ✓ | — | — | — | — | — | Incomplete | Planned |
| Grok / xAI CLI | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| Aider | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| Continue | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| Goose | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| Qwen Code | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| Amazon Q Developer CLI | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| Windsurf | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |
| Zed AI | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Complete | Managed |

## Evidence contract

Each non-empty stage points to a compiled Rust test in `src-tauri/src/client_adapters_tests.rs`. The Rust connector-list path independently parses the same fixture catalog and fails closed to `Planned` when any required stage is absent. Cursor remains Planned because native apply, verify, rollback, and Off-mode fixture proof is intentionally absent.
