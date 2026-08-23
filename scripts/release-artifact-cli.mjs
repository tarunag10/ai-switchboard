import fs from "node:fs";
import path from "node:path";
import { selectReleaseArtifact } from "./release-artifact-contract.mjs";

export function selectReleaseArtifactFromDirectory(
  directory,
  expectedVersion,
  fileSystem = fs,
) {
  const candidates = fileSystem
    .readdirSync(directory)
    .filter((name) => name.toLowerCase().endsWith(".dmg"))
    .map((name) => {
      const candidatePath = path.join(directory, name);
      const stats = fileSystem.statSync(candidatePath);
      return {
        name,
        path: candidatePath,
        regularFile: stats.isFile(),
        mtimeMs: stats.mtimeMs,
      };
    });
  const result = selectReleaseArtifact(candidates, { expectedVersion });
  if (!result.candidate) {
    throw new Error(result.reason ?? "no compatible release artifact found");
  }
  return result.candidate.path;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [directory, expectedVersion] = process.argv.slice(2);
  if (!directory || !expectedVersion) {
    console.error("usage: node scripts/release-artifact-cli.mjs <dmg-directory> <version>");
    process.exit(2);
  }
  try {
    console.log(selectReleaseArtifactFromDirectory(directory, expectedVersion));
  } catch (error) {
    console.error(`release artifact selection failed: ${error.message}`);
    process.exit(1);
  }
}
