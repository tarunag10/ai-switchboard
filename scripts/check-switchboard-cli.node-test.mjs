import assert from "node:assert/strict";
import { test } from "node:test";
import {
  NODE_CONTRACT_SUITES,
  REPO_ROOT,
  runNodeContractSuites,
} from "./check-switchboard-cli-suites.mjs";

test("runs the exact packed CLI suites without invoking the parent check recursively", () => {
  const calls = [];
  const result = runNodeContractSuites((command, args, options) => {
    calls.push({ command, args, options });
    return { status: 0 };
  });

  assert.deepEqual(result, { ok: true, code: 0 });
  assert.deepEqual(NODE_CONTRACT_SUITES, [
    "scripts/npm-pack-contract.node-test.mjs",
    "scripts/native-cli-bridge.node-test.mjs",
  ]);
  assert.deepEqual(
    calls.map(({ command, args, options }) => ({
      command,
      args,
      cwd: options.cwd,
      env: options.env,
      shell: options.shell,
      stdio: options.stdio,
    })),
    NODE_CONTRACT_SUITES.map((suitePath) => ({
      command: process.execPath,
      args: ["--test", suitePath],
      cwd: REPO_ROOT,
      env: process.env,
      shell: false,
      stdio: "inherit",
    })),
  );
  for (const call of calls) {
    assert.doesNotMatch(call.args.join(" "), /check-switchboard-cli/);
    assert.notEqual(call.command, "npm");
    assert.notEqual(call.command, "npm.cmd");
  }
});

test("stops at the first failing suite and propagates its exit status", () => {
  const calls = [];
  const result = runNodeContractSuites((command, args) => {
    calls.push({ command, args });
    return { status: calls.length === 1 ? 23 : 0 };
  });

  assert.deepEqual(result, {
    ok: false,
    code: 23,
    message:
      "Node contract suite failed: scripts/npm-pack-contract.node-test.mjs",
  });
  assert.equal(calls.length, 1);
});

test("uses a stable content-free error when a suite cannot start", () => {
  const result = runNodeContractSuites(() => ({
    error: new Error("private process detail"),
    status: null,
  }));

  assert.deepEqual(result, {
    ok: false,
    code: 1,
    message:
      "could not start Node contract suite: scripts/npm-pack-contract.node-test.mjs",
  });
  assert.doesNotMatch(result.message, /private process detail/);
});
