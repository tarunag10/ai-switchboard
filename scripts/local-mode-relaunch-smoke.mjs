import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const appPath = "/Applications/Mac AI Switchboard.app";
const appName = "Mac AI Switchboard";
const appProcess = "mac-ai-switchboard";
const interceptPort = "6767";
const proxyPort = "6768";
const confirm = process.argv.includes("--confirm");
const summaryPath = "dist/local-mode-relaunch-smoke-summary.md";
const jsonPath = "dist/local-mode-relaunch-smoke-summary.json";
const configPath = path.join(
  os.homedir(),
  "Library",
  "Application Support",
  appName,
  "config",
  "client-setup.json",
);

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    timeout: options.timeout ?? 10_000,
    ...options,
  });
  return {
    command: [command, ...args].join(" "),
    status: result.status,
    stdout: result.stdout?.trim() ?? "",
    stderr: result.stderr?.trim() ?? "",
    ok: result.status === 0,
  };
}

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function isListening(port) {
  return run("lsof", ["-nP", `-iTCP:${port}`, "-sTCP:LISTEN"]).ok;
}

function processLine(pattern) {
  const result = run("pgrep", ["-fl", pattern]);
  return result.ok ? result.stdout : null;
}

function waitFor(predicate, timeoutMs, intervalMs = 500) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (predicate()) return true;
    sleep(intervalMs);
  }
  return false;
}

function quitInstalledApp() {
  run("osascript", ["-e", `tell application "${appName}" to quit`], { timeout: 5_000 });
  waitFor(() => !processLine(appProcess), 8_000);
  if (processLine(appProcess)) {
    run("pkill", ["-x", appProcess]);
    waitFor(() => !processLine(appProcess), 5_000);
  }
}

function stopProxyIfOwned() {
  const proxy = processLine(`headroom proxy --port ${proxyPort}`);
  if (proxy) {
    run("pkill", ["-f", `headroom proxy --port ${proxyPort}`]);
    waitFor(() => !isListening(proxyPort), 5_000);
  }
}

function openInstalledApp() {
  const opened = run("open", ["-a", appName]);
  if (!opened.ok) return opened;
  const appReady = waitFor(() => processLine(appProcess), 15_000);
  return {
    ...opened,
    ok: opened.ok && appReady,
    stdout: appReady ? "installed app process is ready" : opened.stdout,
    stderr: appReady ? opened.stderr : "installed app process did not become ready",
  };
}

function loadConfig() {
  if (!fs.existsSync(configPath)) {
    return {};
  }
  return JSON.parse(fs.readFileSync(configPath, "utf8"));
}

function writeMode(mode) {
  fs.mkdirSync(path.dirname(configPath), { recursive: true });
  const config = loadConfig();
  config.switchboardMode = mode;
  fs.writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
}

function recordModeResult(mode) {
  quitInstalledApp();
  stopProxyIfOwned();
  writeMode(mode);
  const launch = openInstalledApp();
  const appRunning = Boolean(processLine(appProcess));
  const interceptListening = isListening(interceptPort);
  const proxyListening = isListening(proxyPort);
  const persistedMode = loadConfig().switchboardMode ?? null;
  const pass =
    launch.ok &&
    appRunning &&
    persistedMode === mode &&
    !interceptListening &&
    !proxyListening;

  return {
    mode,
    pass,
    launchOk: launch.ok,
    appRunning,
    interceptListening,
    proxyListening,
    persistedMode,
    appProcess: processLine(appProcess),
    proxyProcess: processLine(`headroom proxy --port ${proxyPort}`),
  };
}

if (!confirm) {
  console.error(
    "Refusing to relaunch the installed app without --confirm. This smoke backs up and restores client-setup.json.",
  );
  process.exit(2);
}

if (!fs.existsSync(appPath)) {
  console.error(`Installed app is missing: ${appPath}`);
  process.exit(1);
}

const generatedAt = new Date().toISOString();
const originalExists = fs.existsSync(configPath);
const originalConfig = originalExists ? fs.readFileSync(configPath, "utf8") : null;
const backupPath = `${configPath}.mode-relaunch-smoke-${generatedAt.replace(/[:.]/g, "")}.bak`;
if (originalExists) {
  fs.copyFileSync(configPath, backupPath);
}

let results = [];
let restored = false;
try {
  results = [recordModeResult("off"), recordModeResult("rtk")];
} finally {
  quitInstalledApp();
  stopProxyIfOwned();
  if (originalExists) {
    fs.writeFileSync(configPath, originalConfig);
  } else if (fs.existsSync(configPath)) {
    fs.rmSync(configPath);
  }
  restored = true;
  openInstalledApp();
}

const passed = results.every((result) => result.pass) && restored;
const payload = {
  generatedAt,
  kind: "mac_ai_switchboard.local_mode_relaunch_smoke",
  releaseGateEvidence: false,
  appPath,
  configPath,
  backupPath: originalExists ? backupPath : null,
  restored,
  passed,
  modes: results,
};

const summary = `# Local Mode Relaunch Smoke Summary

Generated: ${generatedAt}

- Evidence kind: local installed mode relaunch check
- Release gate evidence: no
- App path: ${appPath}
- Config path: ${configPath}
- Config backed up: ${originalExists ? "yes" : "no"}
- Config restored: ${restored ? "yes" : "no"}
- Overall result: ${passed ? "pass" : "fail"}

${results
  .map(
    (result) => `## ${result.mode}

- Pass: ${result.pass ? "yes" : "no"}
- Launch ok: ${result.launchOk ? "yes" : "no"}
- App process running: ${result.appRunning ? "yes" : "no"}
- Intercept listener ${interceptPort}: ${result.interceptListening ? "listening" : "not listening"}
- Headroom proxy ${proxyPort}: ${result.proxyListening ? "listening" : "not listening"}
- Persisted mode after relaunch: ${result.persistedMode ?? "unknown"}
`,
  )
  .join("\n")}
This smoke proves the unsigned/ad-hoc installed app can relaunch with saved Off and RTK-only modes without starting the Headroom proxy. It does not prove signed/notarized public release readiness.
`;

fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, summary);
fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);

console.log("Local mode relaunch smoke summary written.");
console.log(`Summary written: ${summaryPath}`);
console.log(`JSON written: ${jsonPath}`);

if (!passed) {
  process.exit(1);
}
