import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

const REGISTRY_PATH = "fixtures/god-file-registry.json";

export function loadGodFileRegistry(root = process.cwd()) {
  const absolute = path.join(root, REGISTRY_PATH);
  if (!fs.existsSync(absolute)) {
    throw new Error(`missing god file registry at ${REGISTRY_PATH}`);
  }
  const registry = JSON.parse(fs.readFileSync(absolute, "utf8"));
  if (!Array.isArray(registry.godFiles)) {
    throw new Error("god file registry must define godFiles array");
  }
  return registry;
}

export function godFilePathSet(registry) {
  return new Set(registry.godFiles.map((entry) => entry.path));
}

export function watchlistPathSet(registry) {
  return new Set((registry.watchlist ?? []).map((entry) => entry.path));
}

export function trackedOversizePathSet(registry) {
  return new Set([...godFilePathSet(registry), ...watchlistPathSet(registry)]);
}

export function measureFile(root, relativePath) {
  const absolute = path.join(root, relativePath);
  const bytes = fs.statSync(absolute).size;
  const lines = Number(
    execFileSync("/usr/bin/wc", ["-l", absolute], { encoding: "utf8" })
      .trim()
      .split(/\s+/)[0],
  );
  return { lines, bytes };
}

export function evaluateGodFileEntry(root, entry) {
  const measured = measureFile(root, entry.path);
  const lineCeiling = entry.baselineLines + entry.maxGrowthLines;
  const growthLines = measured.lines - entry.baselineLines;
  const withinGrowth = measured.lines <= lineCeiling;
  return {
    id: entry.id,
    path: entry.path,
    domain: entry.domain,
    splitSlice: entry.splitSlice,
    baselineLines: entry.baselineLines,
    baselineBytes: entry.baselineBytes,
    maxGrowthLines: entry.maxGrowthLines,
    measuredLines: measured.lines,
    measuredBytes: measured.bytes,
    growthLines,
    lineCeiling,
    withinGrowth,
  };
}

export function evaluateGodFileRegistry(root = process.cwd()) {
  const registry = loadGodFileRegistry(root);
  const allEntries = [
    ...registry.godFiles.map((entry) => ({ tier: "god", entry })),
    ...(registry.watchlist ?? []).map((entry) => ({ tier: "watchlist", entry })),
  ];
  const paths = allEntries.map(({ entry }) => entry.path);
  const duplicates = paths.filter((item, index) => paths.indexOf(item) !== index);
  if (duplicates.length > 0) {
    throw new Error(`duplicate god file paths: ${[...new Set(duplicates)].join(", ")}`);
  }

  const entries = allEntries.map(({ tier, entry }) => {
    const absolute = path.join(root, entry.path);
    if (!fs.existsSync(absolute)) {
      throw new Error(`tracked file missing on disk: ${entry.path}`);
    }
    for (const field of [
      "id",
      "path",
      "domain",
      "splitSlice",
      "baselineLines",
      "baselineBytes",
      "maxGrowthLines",
      "reason",
    ]) {
      if (!(field in entry)) {
        throw new Error(`registry entry ${entry.id ?? entry.path} missing ${field}`);
      }
    }
    return { tier, ...evaluateGodFileEntry(root, entry) };
  });

  return {
    registry,
    entries,
    violations: entries.filter((entry) => !entry.withinGrowth),
  };
}

export { REGISTRY_PATH };
