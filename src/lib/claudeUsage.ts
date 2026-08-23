import { invoke } from "@tauri-apps/api/core";

export interface ClaudeUsageWindow {
  utilization: number;
  resetsAt: string;
}

export interface ClaudeExtraUsage {
  isEnabled: boolean;
  monthlyLimit: number | null;
  usedCredits: number | null;
  utilization: number | null;
}

export interface ClaudeUsage {
  fiveHour: ClaudeUsageWindow | null;
  sevenDay: ClaudeUsageWindow | null;
  extraUsage: ClaudeExtraUsage | null;
}

const record = (value: unknown): Record<string, any> =>
  value && typeof value === "object" ? value as Record<string, any> : {};

function normalizeWindow(value: unknown): ClaudeUsageWindow | null {
  const data = record(value);
  return typeof data.utilization === "number" && typeof (data.resetsAt ?? data.resets_at) === "string"
    ? { utilization: data.utilization, resetsAt: data.resetsAt ?? data.resets_at }
    : null;
}

export function normalizeClaudeUsage(raw: unknown): ClaudeUsage {
  const data = record(raw);
  const extra = record(data.extraUsage ?? data.extra_usage);
  const hasExtra = Object.keys(extra).length > 0;
  return {
    fiveHour: normalizeWindow(data.fiveHour ?? data.five_hour),
    sevenDay: normalizeWindow(data.sevenDay ?? data.seven_day),
    extraUsage: hasExtra
      ? {
          isEnabled: extra.isEnabled ?? extra.is_enabled === true,
          monthlyLimit: typeof (extra.monthlyLimit ?? extra.monthly_limit) === "number" ? extra.monthlyLimit ?? extra.monthly_limit : null,
          usedCredits: typeof (extra.usedCredits ?? extra.used_credits) === "number" ? extra.usedCredits ?? extra.used_credits : null,
          utilization: typeof extra.utilization === "number" ? extra.utilization : null,
        }
      : null,
  };
}

export async function loadClaudeUsage(): Promise<ClaudeUsage> {
  return normalizeClaudeUsage(await invoke("get_claude_usage"));
}
