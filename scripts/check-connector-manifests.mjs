#!/usr/bin/env node
import fs from "node:fs";

const manifestPath = "connectors/manifest.json";
const schemaPath = "schemas/connector.schema.json";
const rustPaths = [
  "src-tauri/src/client_adapters.rs",
  "src-tauri/src/client_connector_status.rs",
  "src-tauri/src/client_connectors.rs",
];
const frontendPath = "src/lib/plannedConnectors.ts";

const allowedStatus = new Set(["managed", "guided", "detected", "planned", "unsupported"]);
const allowedCategory = new Set(["cli", "editor", "agent", "runtime"]);
const managedLifecycleTerms = [
  ["backup", "back up"],
  ["verify", "verification"],
  ["rollback", "roll back", "restore"],
  ["off mode", "off cleanup"],
];

function fail(message) {
  console.error(message);
  process.exitCode = 1;
}

function read(path) {
  return fs.readFileSync(path, "utf8");
}

function parseManifest() {
  if (!fs.existsSync(schemaPath)) fail(`missing schema: ${schemaPath}`);
  const manifests = JSON.parse(read(manifestPath));
  if (!Array.isArray(manifests)) fail("connectors/manifest.json must be an array");
  const seen = new Set();
  for (const connector of manifests) {
    for (const field of [
      "id",
      "name",
      "category",
      "support_status",
      "detection",
      "automation_gates",
      "manual_workflow",
    ]) {
      if (!(field in connector)) fail(`${connector.id ?? "unknown"} missing ${field}`);
    }
    if (!/^[a-z0-9_]+$/.test(connector.id)) fail(`invalid connector id: ${connector.id}`);
    if (seen.has(connector.id)) fail(`duplicate connector id: ${connector.id}`);
    seen.add(connector.id);
    if (!allowedStatus.has(connector.support_status)) {
      fail(`${connector.id} has invalid support_status ${connector.support_status}`);
    }
    if (!allowedCategory.has(connector.category)) {
      fail(`${connector.id} has invalid category ${connector.category}`);
    }
    if (!Array.isArray(connector.automation_gates) || connector.automation_gates.length === 0) {
      fail(`${connector.id} must define automation_gates`);
    }
    if (!Array.isArray(connector.manual_workflow) || connector.manual_workflow.length === 0) {
      fail(`${connector.id} must define manual_workflow`);
    }
    if (connector.support_status === "managed") {
      const gateText = connector.automation_gates.join(" ").toLowerCase();
      for (const variants of managedLifecycleTerms) {
        if (!variants.some((term) => gateText.includes(term))) {
          fail(`${connector.id} managed automation_gates must mention ${variants[0]}`);
        }
      }
    }
  }
  return manifests;
}

function rustConnectorIds() {
  const source = rustPaths.map(read).join("\n");
  const ids = new Set();
  for (const block of source.matchAll(/ManagedClientSpec\s*\{[\s\S]*?id:\s*"([^"]+)"/g)) {
    ids.add(block[1]);
  }
  for (const block of source.matchAll(/PlannedClientSpec\s*\{[\s\S]*?id:\s*"([^"]+)"/g)) {
    ids.add(block[1]);
  }
  return ids;
}

function frontendConnectorIds() {
  const source = read(frontendPath);
  const ids = new Set();
  for (const block of source.matchAll(/\{\s*id:\s*"([^"]+)"/g)) {
    const id = block[1];
    if (!["detect", "dryRunDiff", "backup", "apply", "verify", "rollback", "offCleanup"].includes(id)) {
      ids.add(id);
    }
  }
  ids.add("claude_code");
  ids.add("codex");
  return ids;
}

const manifests = parseManifest();
const manifestIds = new Set(manifests.map((connector) => connector.id));
const rustIds = rustConnectorIds();
const frontendIds = frontendConnectorIds();

for (const id of rustIds) {
  if (!manifestIds.has(id)) fail(`Rust connector missing manifest: ${id}`);
}
for (const id of frontendIds) {
  if (!manifestIds.has(id)) fail(`frontend connector missing manifest: ${id}`);
}
for (const id of manifestIds) {
  if (!rustIds.has(id)) fail(`manifest connector missing Rust registry entry: ${id}`);
  if (!frontendIds.has(id)) fail(`manifest connector missing frontend registry entry: ${id}`);
}

if (process.exitCode) process.exit(process.exitCode);
console.log(`Connector manifests validated (${manifests.length} connectors).`);
