import fs from "node:fs";
import path from "node:path";

export function inspectPublicArtifact(filePath, fileSystem = fs) {
  const normalized = typeof filePath === "string" ? filePath.trim() : "";
  if (!normalized) {
    return { provided: false, ok: true, path: null, reason: null };
  }
  if (!path.isAbsolute(normalized)) {
    return { provided: true, ok: false, path: normalized, reason: "public artifact path must be absolute" };
  }
  if (!normalized.toLowerCase().endsWith(".dmg")) {
    return { provided: true, ok: false, path: normalized, reason: "public artifact path must point to a .dmg file" };
  }
  let stat;
  try {
    stat = fileSystem.statSync(normalized);
  } catch {
    return { provided: true, ok: false, path: normalized, reason: "public artifact file does not exist" };
  }
  if (!stat.isFile()) {
    return { provided: true, ok: false, path: normalized, reason: "public artifact path must point to a regular file" };
  }
  return { provided: true, ok: true, path: normalized, reason: null };
}
