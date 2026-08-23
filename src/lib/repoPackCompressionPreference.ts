import { invoke } from "@tauri-apps/api/core";
import type { RepoPackCompressionMode } from "./repoIntelligence";
import {
  canActivateChonkifyRepoPack,
} from "./chonkifyPromotionGate";
import {
  loadRepoPackCompressionPreference,
  repoPackCompressionPreferenceEvent,
  repoPackCompressionPreferenceKey,
} from "./repoIntelligence";

export interface NativeRepoPackCompressionPreference {
  schemaVersion: number;
  requestedMode: RepoPackCompressionMode;
  effectiveMode: RepoPackCompressionMode;
  blocked: boolean;
  gateVerdict: string;
  evidenceClass: string;
  stored: boolean;
  updatedAt: string;
}

export async function loadNativeRepoPackCompressionPreference(): Promise<NativeRepoPackCompressionPreference> {
  return invoke<NativeRepoPackCompressionPreference>("get_repo_pack_compression_preference");
}

export async function loadAuthoritativeRepoPackCompressionPreference(): Promise<NativeRepoPackCompressionPreference> {
  let preference = await loadNativeRepoPackCompressionPreference();
  if (preference.stored) {
    window.localStorage.removeItem(repoPackCompressionPreferenceKey);
    return preference;
  }
  if (loadRepoPackCompressionPreference() !== "chonkify" || !canActivateChonkifyRepoPack()) {
    return preference;
  }
  const migrated = await saveNativeRepoPackCompressionPreference("chonkify");
  window.localStorage.removeItem(repoPackCompressionPreferenceKey);
  return migrated;
}

export async function saveNativeRepoPackCompressionPreference(
  mode: RepoPackCompressionMode,
): Promise<NativeRepoPackCompressionPreference> {
  const preference = await invoke<NativeRepoPackCompressionPreference>("set_repo_pack_compression_preference", { mode });
  window.dispatchEvent(new Event(repoPackCompressionPreferenceEvent));
  return preference;
}
