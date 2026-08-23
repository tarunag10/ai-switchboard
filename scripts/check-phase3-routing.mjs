import { spawnSync } from "node:child_process";

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
for (const [index, item] of commands.entries()) {
  console.log(`[phase3 ${index + 1}/${commands.length}] ${item.label}`);
  const result = spawnSync(item.command, item.args, {
    cwd,
    env: process.env,
    stdio: "inherit",
    timeout: timeoutMs,
  });
  if (result.error) {
    console.error(`[phase3] ${item.label} failed: ${result.error.message}`);
    process.exit(1);
  }
  if (result.status !== 0) {
    console.error(`[phase3] ${item.label} exited with status ${result.status ?? "signal"}`);
    process.exit(result.status ?? 1);
  }
}
console.log(`[phase3] passed ${commands.length} suites`);
