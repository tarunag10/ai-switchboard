# Comprehensive compression rollback and Off mode

Rollup for C0–C5 features. Switchboard-owned surfaces only; third-party tools remain user-managed.

## C0 — Product shell

| Feature | Switchboard-owned | Backup | Off mode | Rollback Center |
| --- | --- | --- | --- | --- |
| Compression dashboard | Read model in `compressionDashboard.ts` | N/A (derived) | Hidden when no sources | N/A |
| Max compression activation | `maxCompressionActivation.ts` + local receipts | Optimization engine receipts in localStorage | Disables activatable engines only | Managed optimization receipts |
| Doctor compression playbook | `doctorCompressionPlaybook.ts` | N/A | Playbook stages show Off guidance | Doctor repair actions |
| Agent Session checklist | `agentSessionCompressionChecklist.ts` | N/A | Checklist optional | N/A |

## C1 — Headroom depth

| Feature | Switchboard-owned | Backup | Off mode | Rollback Center |
| --- | --- | --- | --- | --- |
| Compression profiles | `compression_profiles.json` in app storage | Settings export | Cleared on Off when user chooses cleanup | `compression_profiles` row |
| Provider upstream | `provider-upstream-profiles.json` | Settings card copy | Env removed on Headroom restart after clear | Upstream override row |
| Content-class breakdown | Runtime `/stats` parse only | N/A | Not shown without Headroom | N/A |
| Tool-result/history toggles | Compression profile advanced flags | Profile preset backup | Defaults restored with profile clear | Profile row |

## C2 — Context and shell

| Feature | Switchboard-owned | Backup | Off mode | Rollback Center |
| --- | --- | --- | --- | --- |
| Chonkify packs | Repo Intelligence CLI receipts | User repo unchanged | `--compression chonkify` opt-in only | Index clear |
| Session budget | Agent Session localStorage | N/A | Budget ignored when session not started | N/A |
| Repo Memory MCP bounds | MCP schema defaults | N/A | MCP optional | MCP uninstall row |
| RTK presets | `rtk-presets.mjs` env blocks | User shell backup | RTK Off in Switchboard mode | RTK integration row |
| Codex concurrency guard | Banner + doc link only | N/A | Banner only in Full/Headroom | N/A |

## C3 — Cache and measurement

| Feature | Switchboard-owned | Backup | Off mode | Rollback Center |
| --- | --- | --- | --- | --- |
| Exact cache | `semantic-cache.sqlite3` + state json | N/A | Reads/writes disabled | Clear cache action |
| Namespace inspector | Stats table (no prompts) | N/A | Hidden when cache disabled | Namespace clear phrase |
| Semantic v2 | Opt-in flag in state json | N/A | Default off | Disable v2 toggle |
| Benchmarks | `benchmarks/fixtures.json` | N/A | N/A | N/A |
| Provider-billed sampling | localStorage opt-in | N/A | Never runs without consent | N/A |

## C4 — Engine promotion

| Feature | Switchboard-owned | Backup | Off mode | Rollback Center |
| --- | --- | --- | --- | --- |
| leanctx shadow | Sidecar env only | User LEANCTX paths | Shadow only; live routing blocked | Sidecar disable |
| PXPipe | Promotion fixture + gate | N/A | Experimental blocked | N/A |
| LLMLingua-2 | Requirements doc | User model weights | Blocked activation | N/A |

## C5 — Coverage

| Feature | Switchboard-owned | Backup | Off mode | Rollback Center |
| --- | --- | --- | --- | --- |
| BYOK dossier | `plannedConnectors.ts` copy | User upstream keys | Doctor warns when misconfigured | Upstream clear |
| LiteLLM wizard | Gateway profile receipts | User LiteLLM files | Guided only; no writes | User removes env |
| cc-switch reconciler | `headroom-advanced-settings.json` | Settings card | Default off | Toggle off + restart |
| Connector promotion | `connectorPromotionGate.ts` | Connector backups | Managed routing repair | Connector rollback rows |

## Off mode cleanup order

1. Stop Headroom runtime and disable managed connector routing.
2. Clear Switchboard-owned compression profile and upstream overrides if requested.
3. Disable exact cache and clear namespaces when the user confirms.
4. Leave user-managed LiteLLM, gateway, and model weights untouched.
