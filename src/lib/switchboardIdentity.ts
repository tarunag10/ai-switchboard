export enum SwitchboardIdentitySlug {
  AiSwitchboard = "ai-switchboard",
  LegacyMacAiSwitchboard = "mac-ai-switchboard",
  LegacyHeadroom = "headroom",
}

export const SWITCHBOARD_ROUTING_FILE = "ai-switchboard-routing.md";
export const LEGACY_SWITCHBOARD_ROUTING_FILE = "mac-ai-switchboard-routing.md";

export const SWITCHBOARD_DRY_RUN_BACKUP_SUFFIX = ".ai-switchboard.bak";
export const LEGACY_SWITCHBOARD_DRY_RUN_BACKUP_SUFFIX = ".mac-ai-switchboard.bak";

export const SWITCHBOARD_MARKER_PREFIXES = [
  SwitchboardIdentitySlug.AiSwitchboard,
  SwitchboardIdentitySlug.LegacyMacAiSwitchboard,
  SwitchboardIdentitySlug.LegacyHeadroom,
] as const;

export function switchboardMarkerIdVariants(blockId: string): string[] {
  return SWITCHBOARD_MARKER_PREFIXES.map((prefix) => `${prefix}:${blockId}`);
}

export function switchboardManagedMarkerId(blockId: string): string {
  return `${SwitchboardIdentitySlug.AiSwitchboard}:${blockId}`;
}

export function switchboardDryRunBackupPath(target: string): string {
  return `${target}${SWITCHBOARD_DRY_RUN_BACKUP_SUFFIX}`;
}

export function switchboardRoutingPath(configDir: string): string {
  const trimmed = configDir.replace(/\/$/, "");
  return `${trimmed}/${SWITCHBOARD_ROUTING_FILE}`;
}

export function switchboardRoutingPaths(configDir: string): string[] {
  const trimmed = configDir.replace(/\/$/, "");
  return [
    `${trimmed}/${SWITCHBOARD_ROUTING_FILE}`,
    `${trimmed}/${LEGACY_SWITCHBOARD_ROUTING_FILE}`,
  ];
}

export function footprintMarkerRecognitionNote(): string {
  return "ai-switchboard:, mac-ai-switchboard:, and headroom: marker blocks are recognized.";
}
