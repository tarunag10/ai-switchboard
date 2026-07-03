import test from "node:test";
import assert from "node:assert/strict";

import {
  buildRtkPresetDecision,
  detectFrameworkPreset,
  filterFrameworkOutput,
  frameworkPresets,
} from "./rtk-presets.mjs";

test("defines the requested RTK framework presets", () => {
  assert.deepEqual(
    frameworkPresets.map((preset) => preset.id).sort(),
    ["cargo", "jest", "pytest", "vitest"],
  );
  for (const preset of frameworkPresets) {
    assert.ok(preset.dropPatterns.length > 0, `${preset.id} has drop patterns`);
    assert.ok(preset.preservePatterns.length > 0, `${preset.id} has preserve patterns`);
  }
});

test("auto-detects presets from commands, files, manifests, and output", () => {
  assert.equal(detectFrameworkPreset({ command: "npm run test -- --run" }), "vitest");
  assert.equal(detectFrameworkPreset({ files: ["jest.config.ts", "src/foo.test.js"] }), "jest");
  assert.equal(detectFrameworkPreset({ manifests: ["[tool.pytest.ini_options]\naddopts = -q"] }), "pytest");
  assert.equal(detectFrameworkPreset({ output: "error[E0308]: mismatched types\n --> src/lib.rs:9:5" }), "cargo");
});

test("vitest filter drops passing noise and preserves failure detail", () => {
  const output = `
 ✓ src/lib/a.test.ts > passes
 ✓ src/lib/b.test.ts > also passes
 FAIL  src/lib/math.test.ts > adds
 AssertionError: expected 1 to be 2
 Expected: 2
 Received: 1
   at src/lib/math.test.ts:12:9
 Test Files  1 failed | 2 passed
 Tests  1 failed | 9 passed
`;
  const result = filterFrameworkOutput(output, "vitest");
  assert.match(result.output, /FAIL\s+src\/lib\/math\.test\.ts/);
  assert.match(result.output, /AssertionError/);
  assert.match(result.output, /src\/lib\/math\.test\.ts:12:9/);
  assert.doesNotMatch(result.output, /also passes/);
  assert.ok(result.estimatedTokensSaved > 0);
});

test("jest filter keeps assertion diffs and stack paths", () => {
  const output = `
 PASS src/green.test.js
 console.log noisy
 FAIL src/red.test.js
 ● red › fails
 expect(received).toEqual(expected)
 - Expected
 + Received
   at Object.<anonymous> (src/red.test.js:8:3)
 Test Suites: 1 failed, 1 passed, 2 total
`;
  const result = filterFrameworkOutput(output, "jest");
  assert.match(result.output, /FAIL src\/red\.test\.js/);
  assert.match(result.output, /Expected/);
  assert.match(result.output, /src\/red\.test\.js:8:3/);
  assert.doesNotMatch(result.output, /PASS src\/green/);
});

test("pytest filter keeps traceback headline and short summary", () => {
  const output = `
 collected 48 items
 tests/test_ok.py .... [ 10%]
 ============================= FAILURES =============================
 ___________________________ test_total ___________________________
 >       assert total == 3
 E       assert 2 == 3
 tests/test_total.py:17: AssertionError
 ===================== short test summary info ======================
 FAILED tests/test_total.py::test_total - assert 2 == 3
`;
  const result = filterFrameworkOutput(output, "pytest");
  assert.match(result.output, /FAILURES/);
  assert.match(result.output, /tests\/test_total\.py:17/);
  assert.match(result.output, /FAILED tests\/test_total\.py::test_total/);
  assert.doesNotMatch(result.output, /collected 48/);
});

test("cargo filter drops build progress but keeps compiler diagnostics", () => {
  const output = `
   Compiling app v0.1.0 (/tmp/app)
   Finished test [unoptimized + debuginfo] target(s) in 1.2s
error[E0308]: mismatched types
  --> src/lib.rs:7:5
   |
7  |     "nope"
   |     ^^^^^^ expected i32, found &str
thread 'tests::it_fails' panicked at src/lib.rs:22:9:
assertion failed: left == right
test result: FAILED. 0 passed; 1 failed
`;
  const result = filterFrameworkOutput(output, "cargo");
  assert.match(result.output, /error\[E0308\]/);
  assert.match(result.output, /src\/lib\.rs:7:5/);
  assert.match(result.output, /thread 'tests::it_fails' panicked/);
  assert.doesNotMatch(result.output, /Compiling app/);
});

test("decision includes byte and token savings when filtering output", () => {
  const decision = buildRtkPresetDecision({
    command: "cargo test",
    output: [
      "Compiling dep_a v0.1.0",
      "Compiling dep_b v0.1.0",
      "Compiling dep_c v0.1.0",
      "Compiling dep_d v0.1.0",
      "error: bad",
      " --> src/lib.rs:1:1",
    ].join("\n"),
  });
  assert.equal(decision.selectedPreset, "cargo");
  assert.equal(decision.detectedFramework, "cargo");
  assert.ok(decision.originalBytes > decision.filteredBytes);
  assert.ok(decision.estimatedTokensSaved > 0);
});
