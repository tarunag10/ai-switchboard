import { spawn } from "node:child_process";

const root = new URL("..", import.meta.url);
const cwd = root.pathname;
const cargoManifest = "src-tauri/Cargo.toml";
const suites = [
  "plugin_promotion_gate",
  "dsh_plugin_maturity",
  "install_pending_update",
  "storage::tests",
  "dedicated_cleanup_rollback_removes_managed_launch_agents_only",
].map((filter) => ({
  label: `native ${filter} suite`,
  args: ["test", "--manifest-path", cargoManifest, "--lib", filter],
}));

const timeoutMs = 5 * 60 * 1000;

function runSuite(suite, index) {
  return new Promise((resolve) => {
    const startedAt = Date.now();
    let observedPhase = "starting";
    let settled = false;
    let timedOut = false;
    const child = spawn("cargo", suite.args, {
      cwd,
      env: process.env,
      stdio: ["inherit", "pipe", "pipe"],
    });

    const markPhase = (chunk) => {
      if (/Blocking waiting for file lock/i.test(chunk)) {
        observedPhase = "cargo-lock";
      } else if (/Compiling |Checking |Finished /i.test(chunk)) {
        observedPhase = "compiling";
      } else if (/Running |test result:/i.test(chunk)) {
        observedPhase = "tests";
      }
    };
    child.stdout.on("data", (chunk) => {
      const text = chunk.toString();
      markPhase(text);
      process.stdout.write(text);
    });
    child.stderr.on("data", (chunk) => {
      const text = chunk.toString();
      markPhase(text);
      process.stderr.write(text);
    });

    const finish = (result) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      resolve({ ...result, elapsedMs: Date.now() - startedAt, observedPhase });
    };
    const timeout = setTimeout(() => {
      timedOut = true;
      observedPhase = "timeout";
      child.kill("SIGTERM");
      setTimeout(() => {
        if (!settled) child.kill("SIGKILL");
      }, 5_000).unref();
    }, timeoutMs);
    child.once("error", (error) => finish({ error }));
    child.once("close", (status, signal) => finish({ status, signal, timedOut }));
    console.log(`[phase6 ${index + 1}/${suites.length}] START ${suite.label}`);
  });
}

for (const [index, suite] of suites.entries()) {
  const result = await runSuite(suite, index);
  const elapsed = `${(result.elapsedMs / 1000).toFixed(1)}s`;
  if (result.error) {
    console.error(
      `[phase6] FAIL ${suite.label} phase=${result.observedPhase} elapsed=${elapsed} timeout=${result.timedOut}: ${result.error.message}`,
    );
    process.exit(1);
  }
  if (result.status !== 0) {
    console.error(
      `[phase6] FAIL ${suite.label} phase=${result.observedPhase} elapsed=${elapsed} timeout=${result.timedOut} status=${result.status ?? "signal"}`,
    );
    process.exit(result.status ?? 1);
  }
  console.log(`[phase6] PASS ${suite.label} phase=${result.observedPhase} elapsed=${elapsed}`);
}
console.log(`[phase6] passed ${suites.length} Cargo suites`);
