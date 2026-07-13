import { execFileSync } from "node:child_process";

// Vercel's Ignore Build Step treats exit 0 as "skip this deployment" and exit
// 1 as "continue with the build". Keep the hosted shell on the latest commit
// that can affect it, while avoiding deployments for native-only/docs-only
// slices that otherwise create noisy, unnecessary build notifications.
// Vercel's documented default uses HEAD^; the environment variable is not
// present for every Git integration path, so keep that fallback instead of
// treating every commit as a web change.
const previous = process.env.VERCEL_GIT_PREVIOUS_SHA || "HEAD^";
const current = process.env.VERCEL_GIT_COMMIT_SHA || "HEAD";

let changedFiles;
try {
  changedFiles = execFileSync("git", ["diff", "--name-only", previous, current, "--"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  })
    .split("\n")
    .map((file) => file.trim())
    .filter(Boolean);
} catch (error) {
  console.warn("Vercel build: unable to inspect the commit range; continuing", error?.message || error);
  process.exit(1);
}

const affectsWebShell = (file) =>
  file === "index.html" ||
  file === "package.json" ||
  file === "package-lock.json" ||
  file === "vite.config.ts" ||
  file === "vite.config.js" ||
  file === "vite.config.mjs" ||
  file === "tsconfig.json" ||
  file === "tsconfig.app.json" ||
  file === "tsconfig.node.json" ||
  file === "vercel.json" ||
  file === ".vercelignore" ||
  file.startsWith("src/") ||
  file.startsWith("public/") ||
  file === "connectors/manifest.json" ||
  file === "docs/repo-map/repo-map.json";

if (changedFiles.some(affectsWebShell)) {
  console.log(`Vercel build: ${changedFiles.length} changed file(s) include web-shell inputs; continuing`);
  process.exit(1);
}

console.log(`Vercel build: skipping ${changedFiles.length} native/docs-only changed file(s)`);
process.exit(0);
