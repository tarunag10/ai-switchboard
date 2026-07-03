#!/usr/bin/env node

import fs from "node:fs";
import {
  buildRtkPresetDecision,
  chooseFrameworkPreset,
  filterFrameworkOutput,
  frameworkPresets,
  getFrameworkPreset,
} from "./rtk-presets.mjs";

function parseArgs(argv) {
  const args = {
    json: false,
    rtkPresets: false,
    mode: "auto",
    command: "",
    files: [],
    manifests: [],
    input: undefined,
    preset: undefined,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--json") args.json = true;
    else if (arg === "--rtk-presets") args.rtkPresets = true;
    else if (arg === "--mode") args.mode = argv[++index] ?? "auto";
    else if (arg === "--command") args.command = argv[++index] ?? "";
    else if (arg === "--file") args.files.push(argv[++index] ?? "");
    else if (arg === "--manifest") args.manifests.push(readMaybeFile(argv[++index] ?? ""));
    else if (arg === "--input") args.input = argv[++index];
    else if (arg === "--preset") args.preset = argv[++index];
    else if (arg === "--help" || arg === "-h") args.help = true;
    else throw new Error(`Unknown argument: ${arg}`);
  }
  return args;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help || !args.rtkPresets) {
    printHelp();
    process.exit(args.help ? 0 : 1);
  }

  const output = args.input ? fs.readFileSync(args.input, "utf8") : "";
  const selection = chooseFrameworkPreset({
    mode: args.preset ?? args.mode,
    command: args.command,
    files: args.files,
    manifests: args.manifests,
    output,
  });
  const decision = buildRtkPresetDecision({
    mode: args.preset ?? args.mode,
    command: args.command,
    files: args.files,
    manifests: args.manifests,
    output,
  });
  const filtered =
    output && selection.selectedPreset !== "auto" && selection.selectedPreset !== "none"
      ? filterFrameworkOutput(output, selection.selectedPreset)
      : undefined;
  const report = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    presets: frameworkPresets,
    decision,
    filteredOutput: filtered?.output,
  };

  if (args.json) {
    process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
    return;
  }

  process.stdout.write(`RTK presets: ${frameworkPresets.map((preset) => preset.id).join(", ")}\n`);
  process.stdout.write(`Decision: ${decision.selectedPreset} (${decision.reason})\n`);
  if (decision.estimatedTokensSaved) {
    process.stdout.write(`Estimated saved: ${decision.estimatedTokensSaved} tokens\n`);
  }
  if (filtered?.output) {
    process.stdout.write("\n");
    process.stdout.write(filtered.output);
    process.stdout.write("\n");
  }
}

function readMaybeFile(value) {
  return fs.existsSync(value) ? fs.readFileSync(value, "utf8") : value;
}

function printHelp() {
  process.stdout.write(`Usage:
  node scripts/optimization-report.mjs --rtk-presets [--json]
  node scripts/optimization-report.mjs --rtk-presets --command "vitest run" --input vitest.log
  node scripts/optimization-report.mjs --rtk-presets --preset pytest --input pytest.log

Options:
  --rtk-presets       Print RTK framework preset report.
  --json              Emit JSON.
  --mode auto|none    Select auto detection or disable filtering.
  --preset <id>       Force vitest, jest, pytest, or cargo.
  --command <cmd>     Command string used for auto detection.
  --file <path>       File path signal, repeatable.
  --manifest <path>   Manifest path or inline manifest signal, repeatable.
  --input <path>      Output fixture/log to filter.
`);
}

try {
  main();
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
}

