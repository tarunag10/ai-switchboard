import { invoke } from "@tauri-apps/api/core";
import { mockDashboard } from "./mockData";
import type { DashboardState, SavingsAttributionEvent } from "./types";

export async function loadDashboard(): Promise<DashboardState> {
  try {
    return await invoke<DashboardState>("get_dashboard_state");
  } catch {
    return mockDashboard;
  }
}

export async function loadSavingsAttributionEvents(): Promise<
  SavingsAttributionEvent[]
> {
  try {
    return await invoke<SavingsAttributionEvent[]>(
      "get_savings_attribution_events",
    );
  } catch {
    return [];
  }
}

export function delay(ms: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, ms);
  });
}
