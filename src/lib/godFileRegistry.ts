import registry from "../../fixtures/god-file-registry.json";

export interface GodFileRegistryEntry {
  id: string;
  path: string;
  domain: string;
  splitSlice: string;
  baselineLines: number;
  baselineBytes: number;
  maxGrowthLines: number;
  reason: string;
}

export interface GodFileSplitModule {
  id: string;
  path: string;
  parent: string;
  domain: string;
}

export interface GodFileRegistry {
  schemaVersion: number;
  defaultBudget: {
    maxLines: number;
    maxBytes: number;
  };
  notes: string[];
  godFiles: GodFileRegistryEntry[];
  watchlist?: GodFileRegistryEntry[];
  splitModules?: GodFileSplitModule[];
  originalBaselines?: Record<string, { lines: number; bytes: number }>;
}

export const godFileRegistry = registry as GodFileRegistry;

export function godFileRegistryPaths(): string[] {
  return godFileRegistry.godFiles.map((entry) => entry.path);
}

export function watchlistRegistryPaths(): string[] {
  return (godFileRegistry.watchlist ?? []).map((entry) => entry.path);
}

export function splitModuleRegistryPaths(): string[] {
  return (godFileRegistry.splitModules ?? []).map((entry) => entry.path);
}

export function trackedOversizeRegistryPaths(): string[] {
  return [...godFileRegistryPaths(), ...watchlistRegistryPaths()];
}

export function findGodFileRegistryEntry(path: string): GodFileRegistryEntry | null {
  return godFileRegistry.godFiles.find((entry) => entry.path === path) ?? null;
}

export function godFileLineCeiling(entry: GodFileRegistryEntry): number {
  return entry.baselineLines + entry.maxGrowthLines;
}

export function describeGodFileRegistry(): string {
  const lines = trackedOversizeRegistryPaths().map((filePath) => {
    const entry =
      findGodFileRegistryEntry(filePath) ??
      godFileRegistry.watchlist?.find((item) => item.path === filePath);
    if (!entry) return filePath;
    return `${filePath} (${entry.baselineLines.toLocaleString()} lines baseline, +${entry.maxGrowthLines} growth cap, split ${entry.splitSlice})`;
  });
  return [
    "Tracked god and watchlist files are exempt from the default file-size budget until split work lands.",
    ...lines,
  ].join(" ");
}

export function validateGodFileRegistry(
  input: GodFileRegistry = godFileRegistry,
): string[] {
  const errors: string[] = [];
  const paths = new Set<string>();
  for (const entry of [...input.godFiles, ...(input.watchlist ?? [])]) {
    if (paths.has(entry.path)) {
      errors.push(`duplicate god file path: ${entry.path}`);
    }
    paths.add(entry.path);
    if (entry.maxGrowthLines <= 0) {
      errors.push(`${entry.path} must define a positive maxGrowthLines cap`);
    }
    if (entry.baselineLines <= 0) {
      errors.push(`${entry.path} must define a positive baselineLines value`);
    }
  }
  if (input.defaultBudget.maxLines <= 0 || input.defaultBudget.maxBytes <= 0) {
    errors.push("defaultBudget must define positive maxLines and maxBytes");
  }
  return errors;
}
