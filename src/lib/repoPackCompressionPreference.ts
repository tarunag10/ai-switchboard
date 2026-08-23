import { invoke } from "@tauri-apps/api/core";
import type { RepoPackCompressionMode } from "./repoIntelligence";

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

export async function saveNativeRepoPackCompressionPreference(
  mode: RepoPackCompressionMode,
): Promise<NativeRepoPackCompressionPreference> {
  return invoke<NativeRepoPackCompressionPreference>("set_repo_pack_compression_preference", { mode });
}
