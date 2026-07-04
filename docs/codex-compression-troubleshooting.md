# Codex Compression Troubleshooting

Use this runbook when Codex on macOS stops after opening multiple active chats or goals while Headroom and RTK are enabled.

## Problem

Codex can accumulate large conversation context in each active chat or goal. When several active sessions run at once, requests routed through Headroom may become too large or take too long to compress. The common recoverable Headroom error looks like:

```text
unexpected status 413 Payload Too Large: compression_refused
```

This is different from:

```text
The '' model is not supported when using Codex with a ChatGPT account.
```

The model error is a Codex model/provider configuration issue. Use Doctor to repair Codex setup and then choose a Codex-supported ChatGPT model before retrying.

## Likely Causes

1. Too many active Codex chats or goals with large accumulated context.
2. Parallel Headroom compression work competing for local CPU or memory.
3. Long tool-heavy tasks producing large context before the next model request.
4. A stale Codex provider block or model setting, if the error mentions an unsupported model instead of `compression_refused`.

## Immediate Recovery

1. Stop starting new Codex goals until one active session is stable.
2. Compact or summarize the largest active Codex conversation.
3. Retry Codex normally; oversized Codex turns auto-route around Headroom so Codex can compact or retry with its native flow.
4. Switch Mac AI Switchboard to **RTK only** if several heavy Codex sessions are active at the same time.
5. If the error mentions a model unsupported with a ChatGPT account, run Doctor and use **Repair Codex** instead of treating it as a compression issue.

## Prevention

1. Use **Full optimization** for one main Codex coding session.
2. Use **RTK only** when running several heavy Codex chats or goals in parallel.
3. Compact long-running Codex sessions before starting another active goal.
4. Avoid running multiple noisy build/test/log-heavy tasks through Headroom at the same time.
5. Keep Doctor visible after mode changes; it shows stale fallback bypass state only when manual repair is still useful.
6. Enable Codex history retagging only after reviewing the SQLite backup and restore notes in `docs/recovery.md`.

Switchboard should warn before failure: **Full: one main Codex session**, **RTK only: 2+ heavy sessions**, **Large turns auto-route around 413**, and **Unsupported model: Repair Codex setup**. Use **Switch to RTK only** before opening several heavy active Codex chats or goals.

## Recommended App Behavior

Mac AI Switchboard should keep these behaviors explicit:

1. If a Codex request is large enough to risk `413 compression_refused`, Switchboard routes it around Headroom before the backend refusal path.
2. Doctor should show **Reset Codex** only for stale fallback bypass state, not for normal automatic recovery.
3. **RTK only** should remain the safe fallback for multiple active Codex goals.
4. Unsupported Codex model/provider errors should stay separate from Headroom compression errors.

## Details To Capture If It Repeats

When reporting the issue, include:

1. Exact error text.
2. Number of active Codex chats and goals.
3. Switchboard mode: **Full optimization**, **Headroom only**, **RTK only**, or **Off**.
4. Whether the active tasks were running tests, builds, searches, or reading large logs.
5. Whether Doctor showed Codex bypass, Codex setup repair, or runtime repair.
