import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

import type { HeadroomPricingStatus, RuntimeStatus } from "./types";

const NEEDS_AUTH_KEY = "headroom_urgent_needs_auth_date";
const OPTIMIZATION_BLOCKED_KEY = "headroom_urgent_opt_blocked_date";
const RUNTIME_DOWN_KEY = "headroom_urgent_runtime_down_date";
// Single daily slot for the upgrade nudge: either a usage-based nudge or, when
// no threshold is crossed, a generic reminder. One key keeps the two mutually
// exclusive so a gated free user gets at most one upgrade nudge per ~24h.
const DAILY_NUDGE_KEY = "headroom_urgent_nudge_date";
const NUDGE_REMINDER_TITLE = "Mac AI Switchboard is ready when you are";
const NUDGE_REMINDER_BODY =
  "You're on the free plan. Upgrade to keep the Headroom engine optimizing every prompt.";

// Codex uses a storage key distinct from the Claude gate so a Claude
// notification firing in the same window can't suppress the Codex one.
const CODEX_OPTIMIZATION_BLOCKED_KEY = "headroom_urgent_codex_opt_blocked_date";

const NUDGE_TITLES: Record<number, string> = {
  1: "Heads up: 25% of your weekly Claude usage",
  2: "Halfway there: 35% of your weekly Claude usage",
  3: "Almost paused: 45% of your weekly Claude usage",
};

// Codex shares the same nudge ladder (25/35/45%) as the Claude gate.
const CODEX_NUDGE_TITLES: Record<number, string> = {
  1: "Heads up: 25% of your weekly Codex usage",
  2: "Halfway there: 35% of your weekly Codex usage",
  3: "Almost paused: 45% of your weekly Codex usage",
};

export async function maybeFireUrgentPricingNotifications(
  status: HeadroomPricingStatus,
  options: { localOnlyMode?: boolean } = {}
): Promise<void> {
  if (options.localOnlyMode) return;
  if (await isWindowVisible()) return;

  if (status.needsAuthentication) {
    await fireOncePerDay(
      NEEDS_AUTH_KEY,
      "Mac AI Switchboard needs you to sign in",
      status.gateMessage ||
        "Sign in to Mac AI Switchboard to keep optimization running.",
      "signin"
    );
    return;
  }

  if (!status.optimizationAllowed) {
    await fireOncePerDay(
      OPTIMIZATION_BLOCKED_KEY,
      "Headroom engine optimization is off",
      status.gateMessage ||
        "Your current plan has optimization disabled. Open Mac AI Switchboard to review.",
      "billing"
    );
    return;
  }

  const codex = status.codex;
  if (codex && !codex.optimizationAllowed) {
    await fireOncePerDay(
      CODEX_OPTIMIZATION_BLOCKED_KEY,
      "Headroom engine optimization is off",
      codex.gateMessage ||
        "Codex optimization is paused. Open Mac AI Switchboard to review.",
      "billing"
    );
    return;
  }

  // One upgrade nudge per ~24h for gated free users. When a weekly usage
  // threshold is crossed we show the usage-based copy, otherwise a generic
  // reminder so we never go silent. Claude/Codex already track the weekly
  // window for us, so there's no separate weekly gate here -- the daily key is
  // the only throttle, and it's shared so the two paths can't both fire.
  if (!isGatedFreeAccount(status)) return;

  const usage = pickUsageNudge(status);
  await fireOncePerDay(
    DAILY_NUDGE_KEY,
    usage?.title ?? NUDGE_REMINDER_TITLE,
    usage?.body ?? NUDGE_REMINDER_BODY,
    "billing"
  );
}

// Highest usage nudge currently active across Claude and Codex, or null when
// neither has crossed a threshold. Ties go to Claude.
function pickUsageNudge(
  status: HeadroomPricingStatus
): { title: string; body: string } | null {
  const claudeLevel =
    status.shouldNudge && status.nudgeLevel > 0 ? Math.min(status.nudgeLevel, 3) : 0;
  const codex = status.codex;
  const codexLevel =
    codex && codex.shouldNudge && codex.nudgeLevel > 0
      ? Math.min(codex.nudgeLevel, 3)
      : 0;

  if (claudeLevel === 0 && codexLevel === 0) return null;

  if (codexLevel > claudeLevel) {
    return {
      title: CODEX_NUDGE_TITLES[codexLevel] ?? "Heads up: weekly Codex usage rising",
      body:
        codex!.gateMessage ||
        "The Headroom engine will pause Codex optimization at your weekly cap. Upgrade to keep going.",
    };
  }

  return {
    title: NUDGE_TITLES[claudeLevel] ?? "Heads up: weekly Claude usage rising",
    body:
      status.gateMessage ||
      "The Headroom engine will pause optimization at your weekly usage cap. Upgrade to keep going.",
  };
}

// A gated free account: authenticated, optimization still allowed, but no
// active subscription or trial. Mirrors the backend gate that drives shouldNudge.
function isGatedFreeAccount(status: HeadroomPricingStatus): boolean {
  const account = status.account;
  return (
    !status.needsAuthentication &&
    status.optimizationAllowed &&
    !!account &&
    !account.subscriptionActive &&
    !account.trialActive
  );
}

export async function maybeFireUrgentRuntimeNotification(
  runtime: RuntimeStatus
): Promise<void> {
  if (await isWindowVisible()) return;

  const runtimeDown =
    runtime.installed && !runtime.running && !runtime.starting && !runtime.paused;
  if (!runtimeDown) return;

  const body = runtime.startupErrorHint
    ? `The Headroom engine isn't running. ${runtime.startupErrorHint}`
    : runtime.startupError
    ? `The Headroom engine isn't running: ${runtime.startupError}`
    : "The Headroom engine isn't running. Open Mac AI Switchboard to restart it.";

  await fireOncePerDay(
    RUNTIME_DOWN_KEY,
    "Mac AI Switchboard engine stopped running",
    body,
    "runtime"
  );
}

// Returns true when a notification was actually shown (false when throttled).
async function fireOncePerDay(
  storageKey: string,
  title: string,
  body: string,
  action: string
): Promise<boolean> {
  const today = new Date().toISOString().slice(0, 10);
  if (localStorage.getItem(storageKey) === today) return false;
  try {
    await invoke("show_notification", { title, body, action });
    localStorage.setItem(storageKey, today);
    return true;
  } catch {
    // best-effort
    return false;
  }
}

async function isWindowVisible(): Promise<boolean> {
  return getCurrentWindow()
    .isVisible()
    .catch(() => true);
}
