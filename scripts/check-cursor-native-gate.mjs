#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();

const requiredSignals = [
  {
    label: "cursor native gate module",
    file: "src-tauri/src/cursor_native.rs",
    needles: ["assess_native_schema", "supported: false", "CURSOR_NATIVE_GATE_REASON"],
  },
  {
    label: "cursor native command",
    file: "src-tauri/src/cursor_native_commands.rs",
    needles: ["get_cursor_native_schema_assessment", "surfaces_detected"],
  },
  {
    label: "frontend cursor gate",
    file: "src/lib/cursorNativeGate.ts",
    needles: ["describeCursorNativeGate", "nativeWritesAllowed"],
  },
];

function fail(message) {
  console.error(`cursor native gate check failed: ${message}`);
  process.exit(1);
}

for (const signal of requiredSignals) {
  const absolute = path.join(root, signal.file);
  if (!fs.existsSync(absolute)) {
    fail(`missing ${signal.file}`);
  }
  const contents = fs.readFileSync(absolute, "utf8");
  for (const needle of signal.needles) {
    if (!contents.includes(needle)) {
      fail(`${signal.label} missing needle ${needle} in ${signal.file}`);
    }
  }
}

const libRs = fs.readFileSync(path.join(root, "src-tauri/src/lib.rs"), "utf8");
if (!libRs.includes("get_cursor_native_schema_assessment")) {
  fail("cursor native assessment command is not registered in lib.rs");
}

const fixturePath = path.join(root, "fixtures/cursor-native-gate-evidence.json");
if (!fs.existsSync(fixturePath)) {
  fail("missing fixtures/cursor-native-gate-evidence.json");
}
const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
if (fixture.nativeWritesAllowed !== false) {
  fail("cursor native gate fixture must keep nativeWritesAllowed false");
}

console.log(
  JSON.stringify(
    {
      ok: true,
      nativeWritesAllowed: false,
      signals: requiredSignals.map((signal) => signal.label),
    },
    null,
    2,
  ),
);
