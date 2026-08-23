import { spawn } from "node:child_process";

const root = new URL("..", import.meta.url);
const cwd = root.pathname;
const cargoManifest = "src-tauri/Cargo.toml";
const commands = [
  {
    label: "frontend routing suites",
    command: "npx",
    args: ["vitest", "run", "src/components/ModelRoutingExperimentCard.test.tsx", "src/components/RoutingModelsView.test.tsx", "src/components/InferenceEndpointProfilesCard.test.tsx"],
  },
  ...[
    "inference_endpoint",
    "inference_endpoint_commands",
    "action_policy",
    "model_routing",
    "endpoint_routing",
    "route_plan",
    "cache_compression_policy",
  ].map((filter) => ({
    label: `native ${filter} suite`,
    command: "cargo",
    args: ["test", "--manifest-path", cargoManifest, "--lib", filter, "--", "--test-threads=1"],
  })),
];

const timeoutMs = 5 * 60 * 1000;
function runCommand(item, index) {
  return new Promise((resolve) => {
    const startedAt = Date.now();
    let observedPhase = "starting";
    let settled = false;
    let timedOut = false;
    const child = spawn(item.command, item.args, {
      cwd,
      env: process.env,
      stdio: ["inherit", "pipe", "pipe"],
      detached: process.platform !== "win32",
    });

    const terminate = (signal) => {
      if (process.platform !== "win32" && child.pid) {
        try {
          process.kill(-child.pid, signal);
          return;
        } catch {
          // Fall back to the launcher when a child has already exited.
        }
      }
      child.kill(signal);
    };

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
      const elapsedMs = Date.now() - startedAt;
      resolve({ ...result, elapsedMs, observedPhase });
    };
    const timeout = setTimeout(() => {
      timedOut = true;
      observedPhase = "timeout";
      terminate("SIGTERM");
      setTimeout(() => {
        if (!settled) terminate("SIGKILL");
      }, 5_000).unref();
    }, timeoutMs);
    child.once("error", (error) => finish({ error }));
    child.once("close", (status, signal) =>
      finish({ status, signal, timedOut }),
    );
    console.log(`[phase3 ${index + 1}/${commands.length}] START ${item.label}`);
  });
}

for (const [index, item] of commands.entries()) {
  const result = await runCommand(item, index);
  const elapsed = `${(result.elapsedMs / 1000).toFixed(1)}s`;
  if (result.error) {
    console.error(
      `[phase3] FAIL ${item.label} phase=${result.observedPhase} elapsed=${elapsed}: ${result.error.message}`,
    );
    process.exit(1);
  }
  if (result.status !== 0) {
    console.error(
      `[phase3] FAIL ${item.label} phase=${result.observedPhase} elapsed=${elapsed} timeout=${result.timedOut} status=${result.status ?? "signal"}`,
    );
    process.exit(result.status ?? 1);
  }
  console.log(
    `[phase3] PASS ${item.label} phase=${result.observedPhase} elapsed=${elapsed}`,
  );
}
console.log(`[phase3] passed ${commands.length} suites`);
