import { invoke } from "@tauri-apps/api/core";
import type { SavingsAttributionCounter } from "./types";

export async function loadSavingsAttributionCounters(): Promise<SavingsAttributionCounter[]> {
  return invoke<SavingsAttributionCounter[]>("get_savings_attribution_counters");
}
