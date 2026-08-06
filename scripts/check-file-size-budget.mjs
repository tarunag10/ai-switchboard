#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { statSync } from "node:fs";
import {
  evaluateGodFileRegistry,
  loadGodFileRegistry,
  trackedOversizePathSet,
} from "./lib/god-file-registry.mjs";

const registry = loadGodFileRegistry();
const MAX_LINES = Number(
  process.env.FILE_SIZE_MAX_LINES ?? registry.defaultBudget.maxLines,
);
const MAX_BYTES = Number(
  process.env.FILE_SIZE_MAX_BYTES ?? registry.defaultBudget.maxBytes,
);
const TRACKED_OVERSIZE = trackedOversizePathSet(registry);
const roots = process.argv.slice(2);
const targets = roots.length > 0 ? roots : ["src", "src-tauri/src", "scripts"];
const extensions = new Set([".mjs", ".rs", ".ts", ".tsx"]);

const files = execFileSync("/usr/bin/find", [...targets, "-type", "f"], {
  encoding: "utf8",
})
  .split("\n")
  .filter(Boolean)
  .filter((file) => [...extensions].some((ext) => file.endsWith(ext)));

const oversized = files
  .map((file) => {
    const bytes = statSync(file).size;
    const lines = Number(
      execFileSync("/usr/bin/wc", ["-l", file], { encoding: "utf8" })
        .trim()
        .split(/\s+/)[0],
    );
    return { file, bytes, lines };
  })
  .filter(
    ({ file, bytes, lines }) =>
      !TRACKED_OVERSIZE.has(file) && (lines > MAX_LINES || bytes > MAX_BYTES),
  )
  .sort((a, b) => b.lines - a.lines || b.bytes - a.bytes);

if (oversized.length > 0) {
  console.error(
    `File size budget exceeded: max ${MAX_LINES} lines or ${MAX_BYTES} bytes.`,
  );
  console.error(
    "Register intentional monoliths in fixtures/god-file-registry.json or split the file.",
  );
  for (const item of oversized) {
    console.error(`${item.lines} lines, ${item.bytes} bytes: ${item.file}`);
  }
  process.exit(1);
}

const godReport = evaluateGodFileRegistry();
if (godReport.violations.length > 0) {
  console.error("God file growth budget exceeded:");
  for (const entry of godReport.violations) {
    console.error(
      `${entry.path}: ${entry.measuredLines} lines (ceiling ${entry.lineCeiling})`,
    );
  }
  process.exit(1);
}

console.log(
  `File size budget ok: ${files.length} files checked; ${TRACKED_OVERSIZE.size} tracked oversize files.`,
);
