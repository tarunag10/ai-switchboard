# Live Token X-Ray and Daily AI Usage Briefing Implementation Plan

Status: initial release implemented

Updated: 2026-07-11

## Purpose

Build two connected, local-first product capabilities:

1. **Live Token X-Ray**: a real-time explanation of where agent context goes, what optimization changed, how close a session is to its context limit, and which evidence supports each number.
2. **Daily AI Usage Briefing**: a compact, actionable summary of agent activity, cost, savings, failures, stale indexes, and recommended maintenance across the current day and recent local history.

These should share one normalized analytics pipeline. Token X-Ray is the live/session view; Daily Briefing is the persisted daily rollup and recommendation view.

## Delivery Status

| Slice | Status | Current checkpoint |
| --- | --- | --- |
| 0. Contract and fixture baseline | complete | Versioned, camelCase V1 contracts and frontend normalization tests protect the local boundary. |
| 1. Normalized analytics core | complete | Current-session/current-day evidence is normalized with attribution fingerprints and category separation. |
| 2. Token X-Ray backend | complete | Versioned snapshot, freshness, honest unavailable metrics, source attribution, timeline, and anomalies ship through Tauri. |
| 3. Token X-Ray UI | complete | Dedicated sidebar view presents loading, error, unavailable, provenance, pressure, metrics, source, anomaly, and timeline states. |
| 4. Daily rollup engine | complete | Content-free daily snapshots are atomically persisted locally, listed, pruned after 365 days, and tolerate malformed files. |
| 5. Deterministic recommendations | complete | Local advisory rules are prioritized, capped at three, and deep-link into existing app views without mutation. |
| 6. Daily Briefing UI and export | complete | Briefing, history, secret-free Markdown/JSON copy, and explicit deletion preview/confirmation ship in the tray app. |
| 7. Evidence, documentation, and release gate | complete | Rust/frontend test suites, production build, formatting, and diff hygiene pass; aggregate local-only guard remains an unrelated pre-existing failure. |

The initial release is implemented. It does not authorize analytics network traffic, collection of prompt/response content, or a new telemetry service. Provider details that are not credibly available remain explicitly unavailable rather than being inferred as exact values.

## Product Outcome

AI Switchboard should answer four questions without requiring users to inspect provider dashboards or raw logs:

- What is consuming my context right now?
- What did Switchboard save, and how certain is that claim?
- Am I approaching a context, usage, or cost problem?
- What should I do next?

The implementation must preserve the existing product principles:

- Local-first storage and processing.
- No new analytics or telemetry service.
- Measured, estimated, and inferred values remain visibly distinct.
- Provider/account network calls remain explicit and compatible with local-only mode.
- Missing data produces an honest unavailable state, not a fabricated zero.
- Existing attribution and usage contracts remain the source of truth until a versioned replacement is proven.

## Current Foundations

The app already provides much of the required raw evidence:

- Append-only savings-attribution events and aggregate counters.
- Headroom session and history data.
- RTK daily gain and measured token rows.
- Repo Intelligence token-avoidance estimates.
- Durable estimated evidence for Caveman, Ponytail, MarkItDown, and Compact Chinese.
- Per-agent and per-provider usage events.
- Claude and Codex provider usage-window helpers.
- Session, repo, today, week, month, and lifetime scopes.
- Output-growth, low-savings, and cost-growth anomaly signals.
- Home and Usage surfaces with per-client trends.

The new work should normalize and present this evidence. It should not create parallel counters that can disagree with the savings ledger.

## Scope

### Live Token X-Ray

The first complete release should show:

- Current session input tokens, output tokens, cache-read tokens, cache-write tokens, saved tokens, and effective context consumption.
- Per-agent and per-provider breakdowns where attribution is available.
- Per-source optimization impact for Headroom, RTK, Repo Intelligence, and enabled add-ons.
- Confidence labels: `measured`, `estimated`, `inferred`, and `unavailable`.
- Context-pressure bands with an explanation of the threshold source.
- A chronological event stream for material usage, savings, compaction, fallback, and anomaly events.
- Session comparison against the trailing local baseline when enough history exists.
- Clear caveats when a provider does not expose context-window size, cache metrics, pricing, or request-level attribution.

### Daily AI Usage Briefing

The first complete release should show:

- Requests and active agents for the local day.
- Tokens spent, saved, cached, and avoided, separated by evidence confidence.
- Estimated cost and savings only where a versioned pricing source is available.
- Most expensive agent/provider and highest-context session.
- Repeated failures or anomaly categories.
- Stale Repo Intelligence or Repo Map state affecting recent sessions.
- Connector, runtime, MCP, and add-on health items that require attention.
- Three or fewer prioritized recommendations with evidence and a direct in-app destination.
- A copyable Markdown briefing and a secret-free JSON export.

### Explicit Non-Goals for the First Release

- Cloud synchronization or a hosted analytics dashboard.
- Team-wide aggregation.
- Reading complete prompts, responses, or source files into analytics storage.
- Claiming exact request cost when only subscription-window data exists.
- Automatic model switching or connector mutation from a recommendation.
- OS notifications before the briefing signal quality is validated.
- Long-term retention without an explicit retention preference.

## Information Architecture

### Navigation

- Add **Token X-Ray** as a dedicated detail view reachable from Home and Usage.
- Add **Daily Briefing** as a Home card with a full-detail route.
- Keep the existing savings ledger as the detailed attribution audit surface.
- Deep-link recommendations to Doctor, Mode Inspector, Repo Intelligence, Repo Map, Add-ons, or Usage.

### Token X-Ray Layout

1. Session status: agent, provider, model when known, elapsed time, and data freshness.
2. Context pressure: used/known limit, projected next-turn pressure, and threshold band.
3. Token composition: input, output, cache read/write, saved, and avoided.
4. Optimization impact: source rows with confidence, evidence, and caveat.
5. Timeline: bounded recent events with anomaly markers.
6. Explanation drawer: formulas, missing fields, source timestamps, and data provenance.

### Daily Briefing Layout

1. Plain-language headline summarizing the day.
2. Today cards: requests, spent tokens, saved tokens, and estimated cost.
3. Agent/provider comparison.
4. Attention items: failures, high context pressure, stale repo evidence, or degraded tools.
5. Recommended actions, capped at three.
6. Copy/export controls and retention disclosure.

## Data Contracts

Introduce versioned read models rather than exposing storage-specific structs directly to the frontend.

### `TokenXraySnapshotV1`

```ts
type EvidenceConfidence = "measured" | "estimated" | "inferred" | "unavailable";

type TokenMetricV1 = {
  value: number | null;
  confidence: EvidenceConfidence;
  source: string;
  observedAt: number | null;
  caveat: string | null;
};

type TokenXraySnapshotV1 = {
  schemaVersion: 1;
  generatedAt: number;
  sessionId: string;
  agent: string | null;
  provider: string | null;
  model: string | null;
  freshness: "live" | "recent" | "stale" | "unavailable";
  metrics: {
    inputTokens: TokenMetricV1;
    outputTokens: TokenMetricV1;
    cacheReadTokens: TokenMetricV1;
    cacheWriteTokens: TokenMetricV1;
    savedTokens: TokenMetricV1;
    avoidedTokens: TokenMetricV1;
    estimatedCostUsd: TokenMetricV1;
    estimatedSavingsUsd: TokenMetricV1;
  };
  contextPressure: ContextPressureV1;
  sources: OptimizationImpactV1[];
  timeline: TokenXrayEventV1[];
  anomalies: UsageAnomalyV1[];
};
```

`ContextPressureV1` must distinguish a known provider/model limit from a configured or heuristic limit. If no credible limit exists, the percentage is `null` and the UI shows absolute token evidence only.

### `DailyUsageBriefingV1`

```ts
type DailyUsageBriefingV1 = {
  schemaVersion: 1;
  dayKey: string;
  timezone: string;
  generatedAt: number;
  completeness: "complete" | "partial" | "insufficient-data";
  totals: DailyUsageTotalsV1;
  agents: AgentUsageRollupV1[];
  providers: ProviderUsageRollupV1[];
  attentionItems: BriefingAttentionItemV1[];
  recommendations: BriefingRecommendationV1[];
  evidenceCoverage: EvidenceCoverageV1;
};
```

Every recommendation must include:

- Stable rule ID.
- Severity and priority score.
- Human-readable reason.
- Concrete evidence references.
- Suggested action label.
- Internal destination.
- Whether the action is read-only, advisory, or state-changing.

No state-changing action is executed directly from briefing generation.

## Normalization and Calculation Rules

### Source Precedence

For any overlapping metric:

1. Request/session-level measured event.
2. Durable measured daily event.
3. Provider/runtime aggregate reconciled to the same time window.
4. Evidence-backed estimate.
5. Inferred template or enabled-state estimate.

Lower-confidence sources may supplement a metric but must not overwrite higher-confidence evidence.

### Double-Counting Guard

- Give every normalized contribution a stable source event ID or deterministic fingerprint.
- Track whether a metric describes tokens spent, tokens transformed, tokens saved, or tokens avoided.
- Do not add Repo Intelligence avoided tokens to provider tokens saved as though both were request compression.
- Do not add a provider aggregate and its underlying request rows into the same total.
- Record reconciliation warnings when daily totals differ materially across credible sources.

### Time Boundaries

- Use a persisted IANA timezone preference, defaulting to the current system timezone.
- Store event timestamps in UTC.
- Derive `dayKey` using the selected timezone.
- Rebuild the current and previous day when timezone changes or late events arrive.
- Handle daylight-saving transitions using calendar-day boundaries, not a fixed 24-hour duration.

### Cost

- Version model-pricing metadata and record the pricing version used for every estimate.
- Treat subscription usage windows separately from per-token spend.
- Never display `$0.00` when cost is unknown; display `Unavailable`.
- Preserve historical calculated values or their pricing version so later price changes do not silently rewrite the past.

### Context Pressure

Initial bands:

- Normal: below 60%.
- Elevated: 60% to below 80%.
- High: 80% to below 90%.
- Critical: 90% or greater.

These bands must be centralized and testable. A provider/model-specific known limit takes precedence over a heuristic. Projected pressure must be labelled as an estimate.

## Persistence and Privacy

### Storage

- Continue using app-owned local storage.
- Persist normalized, content-free event facts and daily rollups.
- Do not persist prompts, responses, tool arguments, terminal output, source snippets, file contents, bearer tokens, or API keys.
- Hash or omit repository paths in cross-repository summaries; reveal a path only in the existing local repository-specific UI where already expected.
- Add a schema version and migration path.

### Retention

Default proposal:

- Detailed normalized events: 30 days.
- Daily rollups: 365 days.
- Current session in memory plus durable content-free facts required for the daily rollup.

Expose retention controls in Settings and support immediate local deletion with a preview of affected analytics data. Retention changes must not delete existing savings-ledger evidence unless the user explicitly selects it.

### Local-Only Mode

Token X-Ray and Daily Briefing must work from existing local evidence when local-only mode is enabled. Provider usage-window refresh and remote pricing updates must remain disabled unless separately authorized by existing settings.

## Backend Design

Add focused modules rather than expanding `lib.rs` or a general dashboard module:

- `analytics_normalization.rs`: normalize existing usage and attribution evidence.
- `token_xray.rs`: build the current session snapshot and bounded timeline.
- `daily_briefing.rs`: create/rebuild daily rollups and recommendations.
- `analytics_retention.rs`: retention and deletion preview/execution.
- `analytics_models.rs`: versioned read models and persisted schemas.

Suggested Tauri commands:

- `get_token_xray_snapshot`
- `get_token_xray_timeline`
- `get_daily_usage_briefing`
- `list_daily_usage_briefings`
- `export_daily_usage_briefing`
- `preview_clear_usage_analytics`
- `clear_usage_analytics`

Use an app event such as `token-xray-updated` to refresh the live view. Coalesce updates so high-frequency runtime events do not cause excessive disk writes or frontend renders.

### Initial implementation boundary

The first implementation checkpoint exposes these read-only commands:

- `get_token_xray_snapshot`
- `get_daily_usage_briefing`
- `export_daily_usage_briefing`
- `list_daily_usage_briefings`
- `preview_clear_usage_analytics`
- `clear_usage_analytics`

The initial Rust contracts live in `src-tauri/src/analytics_models.rs` and serialize as camelCase version-1 payloads. Export returns a secret-free `{ briefing, markdown }` payload to the caller; it does not write an export file. Retention preview/clear returns `briefingCount`, `eventCount`, `dayKeys`, `scope`, and an explicit `detail` string. `eventCount` is currently `0` because detailed normalized event facts are not persisted yet; the command never counts snapshots as events or touches the savings ledger.

## Recommendation Engine

Start with deterministic local rules. Do not use an LLM to generate recommendations in the first release.

Initial rules:

| Rule ID | Trigger | Suggested destination |
| --- | --- | --- |
| `context-pressure-high` | Credible session pressure reaches high or critical | Token X-Ray / compact guidance |
| `repeated-route-failure` | Same route failure category repeats within the day | Doctor |
| `repo-index-stale` | A used repository has stale Repo Intelligence evidence | Repo Intelligence |
| `repo-map-stale` | A used repository relies on a stale Repo Map | Repo Map |
| `runtime-degraded` | Headroom/runtime evidence is degraded | Mode Inspector |
| `mcp-unhealthy` | Repo Memory MCP health is stale or failed | Doctor / MCP preparation |
| `low-savings-high-volume` | High token volume with a low credible savings ratio | Usage / optimization mode guidance |
| `output-growth` | Existing output-growth anomaly occurs | Token X-Ray |
| `cost-growth` | Credible daily cost materially exceeds local baseline | Usage |

Rules must include minimum sample requirements and cooldowns. The briefing should prefer one root-cause action over several symptoms and show no more than three recommendations.

## Implementation Slices

### Slice 0: Contract and Fixture Baseline

- Inventory all existing usage, attribution, anomaly, history, provider-window, and pricing inputs.
- Add representative redacted fixtures for Claude, Codex, Headroom, RTK, Repo Intelligence, and add-on evidence.
- Define `TokenXraySnapshotV1` and `DailyUsageBriefingV1` in Rust and TypeScript.
- Document precedence, deduplication, and unknown-value behavior.
- Add a contract check preventing measured/estimated/inferred labels from being dropped.

Done when fixture-backed contracts serialize consistently and existing savings tests remain unchanged.

### Slice 1: Normalized Analytics Core

- Implement normalization and stable event fingerprints.
- Reconcile session and daily evidence without double counting.
- Add time-zone-aware day grouping.
- Add pricing-version and context-limit provenance.
- Expose a diagnostic normalization report for tests and developer builds.

Done when the same fixtures produce deterministic totals across relaunch and aggregation order.

### Slice 2: Token X-Ray Backend

- Build the live session snapshot.
- Add context-pressure calculation.
- Add the bounded timeline and existing anomaly integration.
- Emit coalesced update events.
- Add stale and unavailable states.

Done when backend tests cover mixed confidence, missing model limits, late events, deduplication, and negative-savings anomalies.

### Slice 3: Token X-Ray UI

- Add the dedicated view and navigation entry points.
- Implement metric cards, token composition, source impact, context pressure, timeline, and provenance drawer.
- Provide accessible text equivalents for charts and colors.
- Add loading, empty, partial, stale, and error states.
- Link source rows to the existing savings ledger where useful.

Done when component tests prove confidence/caveat rendering and the view stays usable at menubar dimensions.

### Slice 4: Daily Rollup Engine

- Persist content-free normalized events and daily rollups.
- Rebuild today and handle late events.
- Add baseline comparisons with minimum sample rules.
- Implement retention and migration behavior.
- Add secret-free Markdown and JSON serialization.

Done when day-boundary, timezone, DST, relaunch, retention, and migration tests pass.

### Slice 5: Deterministic Recommendations

- Implement the initial rule registry.
- Add evidence references, severity, priority, cooldown, and root-cause suppression.
- Map actions to internal destinations without executing mutations.
- Limit output to three recommendations.

Done when table-driven tests cover every rule, conflicts, cooldowns, insufficient evidence, and local-only mode.

### Slice 6: Daily Briefing UI and Export

- Add the Home summary card and detail view.
- Show totals, comparisons, attention items, recommendations, and evidence coverage.
- Add copyable Markdown and JSON export.
- Add Settings retention controls and analytics deletion preview.

Done when exports contain no secrets or raw content and every recommendation deep-link resolves.

### Slice 7: Evidence, Documentation, and Release Gate

- Add a fixture-backed local smoke script for Token X-Ray and Daily Briefing.
- Add frontend contract/component checks.
- Add privacy and secret-scanning fixtures.
- Document calculations, confidence, retention, exports, and local-only behavior.
- Add the new checks to `evidence:local` only after runtime is stable.
- Update `README.md`, `docs/plan-status-ledger.md`, and `CHANGELOG.md` as slices ship.

Done when local evidence produces a durable pass/fail summary and release reporting distinguishes shipped proof from remaining live-provider validation.

## Testing Strategy

### Rust Unit Tests

- Metric precedence and confidence preservation.
- Duplicate event rejection.
- Request versus aggregate reconciliation.
- Token category separation.
- Known, heuristic, and unavailable context limits.
- Cost unavailable versus true zero.
- Day keys across timezone and DST boundaries.
- Late-arriving event rebuilds.
- Recommendation thresholds, cooldowns, and conflicts.
- Retention preview and scoped deletion.

### Frontend Tests

- Measured, estimated, inferred, and unavailable visual/text treatment.
- Partial-data and stale-data states.
- Context-pressure accessibility labels.
- Recommendation destination routing.
- Menubar-width overflow and scroll behavior.
- Copy/export error handling.
- No chart-only communication of critical state.

### Integration and Smoke Tests

- Mixed Claude/Codex local fixture day.
- Headroom unavailable with RTK-only evidence.
- No requests today.
- High-volume day with missing pricing.
- Stale Repo Intelligence and unhealthy MCP recommendation flow.
- App relaunch preserves daily totals without duplicating events.
- Local-only mode performs no analytics-related network request.
- Export excludes secret-like fixture values and raw content.

## Validation Commands

Use existing project commands where applicable and add focused scripts during implementation:

```bash
npm run test:desktop
npm run build
npm run evidence:local
git diff --check
```

Proposed focused commands:

```bash
npm run check:token-xray-contract
npm run smoke:token-xray:local
npm run smoke:daily-briefing:local
npm run check:analytics-privacy
```

Until the focused commands are implemented, run the existing baseline after each completed implementation slice:

```bash
npm run test:frontend
npm run test:desktop
npm run build
npm run check:local-only-network
git diff --check
```

Do not add Token X-Ray or Daily Briefing checks to `npm run evidence:local` until their commands produce a durable, content-free pass/fail summary and local-only mode is explicitly covered.

At the initial read-model checkpoint, validate the public command names and serialization with the desktop test suite before adding smoke scripts. Frontend helper tests should target the standalone `src/lib/usageAnalytics.ts` boundary rather than duplicate Tauri contract behavior in view tests.

## Telemetry and Success Criteria

Success must be assessable locally without adding product telemetry.

Release criteria:

- A user can explain the current session's token composition and evidence quality.
- High context pressure is visible before a preventable overflow or compaction failure.
- Daily totals reconcile with the existing savings ledger within documented source boundaries.
- The briefing produces no more than three actionable, evidence-backed recommendations.
- Empty or incomplete evidence never appears as measured zero.
- Local-only network certification continues to pass.
- Exports contain no raw prompts, responses, source content, secrets, or credentials.
- Relaunch does not duplicate events or change historical totals unexpectedly.

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| Conflicting totals across current data sources | Central precedence and reconciliation layer with visible warnings |
| Double counting savings and avoided context | Typed metric categories and stable source fingerprints |
| False precision in cost or context limits | Nullable values, provenance, pricing versions, and confidence labels |
| Excessive disk writes from live events | In-memory snapshot plus coalesced bounded persistence |
| Briefing recommendation fatigue | Three-item cap, cooldowns, root-cause suppression, minimum samples |
| Sensitive content entering analytics | Content-free schema, export allowlist, secret fixtures, privacy gate |
| Large frontend/backend files growing further | Dedicated modules and focused components per slice |
| Feature duplicating the savings ledger | Treat ledger as attribution audit source; X-Ray and Briefing are read models |

## Open Decisions Before Slice 1

1. Whether Token X-Ray should be a permanent sidebar destination or a drill-down from Home/Usage for the first release.
2. Whether detailed analytics retention defaults to 14 or 30 days.
3. Which model context-limit metadata can be shipped and maintained reliably without remote lookup.
4. Whether repository identity in the briefing should use display name, local hash, or an opt-in full path.
5. Whether daily Markdown exports should include local repository names by default.

## Recommended Delivery Order

Ship the shared contracts and Token X-Ray first, then build Daily Briefing on the proven normalized event stream:

1. Contract and fixture baseline.
2. Normalized analytics core.
3. Token X-Ray backend and UI.
4. Daily persistence and rollups.
5. Recommendation engine.
6. Daily Briefing UI, export, and retention controls.
7. Evidence integration and documentation sync.

This order creates user-visible value early while preventing the daily summary from being built on totals that have not yet been reconciled.
