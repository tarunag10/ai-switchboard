import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const NODE_CONTRACT_SUITES = Object.freeze([
  "scripts/npm-pack-contract.node-test.mjs",
  "scripts/native-cli-bridge.node-test.mjs",
]);

function runNodeContractSuites(spawn = spawnSync) {
  for (const suitePath of NODE_CONTRACT_SUITES) {
    const result = spawn(process.execPath, ["--test", suitePath], {
      cwd: REPO_ROOT,
      env: process.env,
      shell: false,
      stdio: "inherit",
    });

    if (result.error) {
      return {
        ok: false,
        code: 1,
        message: `could not start Node contract suite: ${suitePath}`,
      };
    }
    if (result.status !== 0) {
      return {
        ok: false,
        code: result.status ?? 1,
        message: `Node contract suite failed: ${suitePath}`,
      };
    }
  }

  return { ok: true, code: 0 };
}

export { NODE_CONTRACT_SUITES, REPO_ROOT, runNodeContractSuites };
