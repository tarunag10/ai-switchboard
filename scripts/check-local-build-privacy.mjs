import fs from "node:fs";
import path from "node:path";

const truthyValues = new Set(["1", "true", "yes", "on"]);

function truthy(value) {
  return typeof value === "string" && truthyValues.has(value.trim().toLowerCase());
}

const buildFlavor =
  process.env.HEADROOM_BUILD_FLAVOR ||
  process.env.VITE_HEADROOM_BUILD_FLAVOR ||
  "local-free";
const remoteServices =
  truthy(process.env.HEADROOM_REMOTE_SERVICES) ||
  truthy(process.env.VITE_HEADROOM_REMOTE_SERVICES);

if (buildFlavor !== "local-free" || remoteServices) {
  console.log(
    `Skipping local-free privacy scan for build flavor ${buildFlavor} with remote services ${
      remoteServices ? "enabled" : "disabled"
    }.`,
  );
  process.exit(0);
}

const scanRoots = [
  "dist",
  "src-tauri/target/release/bundle",
  "src-tauri/target/release/Mac AI Switchboard.app",
];

const forbiddenStrings = [
  "clarity.ms",
  "www.clarity.ms",
  "app.aptabase.com",
  "aptabase",
  "sentry.io",
  "extraheadroom.com/api",
  "HEADROOM_ACCOUNT_API_BASE_URL",
  "REPLACE_WITH_APTABASE_APP_KEY",
  "REPLACE_WITH_SENTRY_DSN",
  "REPLACE_WITH_CLARITY_PROJECT_ID",
  "checkout.stripe.com",
];

const skippedRoots = [];
const files = [];

function walk(current) {
  const stat = fs.statSync(current);
  if (stat.isDirectory()) {
    for (const entry of fs.readdirSync(current)) {
      walk(path.join(current, entry));
    }
    return;
  }

  if (stat.isFile()) {
    files.push(current);
  }
}

for (const root of scanRoots) {
  if (!fs.existsSync(root)) {
    skippedRoots.push(root);
    continue;
  }
  walk(root);
}

const failures = [];

for (const file of files) {
  const body = fs.readFileSync(file);
  const text = body.toString("utf8");
  for (const forbidden of forbiddenStrings) {
    if (text.includes(forbidden)) {
      failures.push(`${file}: ${forbidden}`);
    }
  }
}

if (failures.length > 0) {
  console.error("Local-free build contains forbidden remote-service strings:");
  console.error(failures.join("\n"));
  process.exit(1);
}

const skipped = skippedRoots.length > 0 ? ` Skipped missing roots: ${skippedRoots.join(", ")}.` : "";
console.log(`Local-free privacy scan passed for ${files.length} files.${skipped}`);
