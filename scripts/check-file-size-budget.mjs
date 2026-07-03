#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { statSync } from "node:fs";

const MAX_LINES = Number(process.env.FILE_SIZE_MAX_LINES ?? 450);
const MAX_BYTES = Number(process.env.FILE_SIZE_MAX_BYTES ?? 40 * 1024);
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
  .filter(({ bytes, lines }) => lines > MAX_LINES || bytes > MAX_BYTES)
  .sort((a, b) => b.lines - a.lines || b.bytes - a.bytes);

if (oversized.length > 0) {
  console.error(
    `File size budget exceeded: max ${MAX_LINES} lines or ${MAX_BYTES} bytes.`,
  );
  for (const item of oversized) {
    console.error(`${item.lines} lines, ${item.bytes} bytes: ${item.file}`);
  }
  process.exit(1);
}

console.log(`File size budget ok: ${files.length} files checked.`);
