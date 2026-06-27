import fs from "node:fs";

const frontendPath = "src/lib/plannedConnectors.ts";
const backendPath = "src-tauri/src/client_adapters.rs";

function readFile(path) {
  return fs.readFileSync(path, "utf8");
}

function uniqueSorted(values) {
  return [...new Set(values)].sort();
}

function extractArrayBody(source, pattern, label) {
  const match = source.match(pattern);
  if (!match) {
    throw new Error(`Could not find ${label}.`);
  }
  return match[1];
}

function splitTopLevelObjects(body, objectStartPattern = /{/g) {
  const starts = [...body.matchAll(objectStartPattern)].map((match) => match.index);
  return starts
    .map((start, index) => {
      const end = index + 1 < starts.length ? starts[index + 1] : body.length;
      return body.slice(start, end);
    })
    .filter((entry) => entry.includes("id:"));
}

function readStringField(block, field) {
  const match = block.match(new RegExp(`\\b${field}:\\s*"([^"]+)"`));
  return match?.[1] ?? null;
}

function countStringArrayField(block, field) {
  const match = block.match(new RegExp(`\\b${field}:\\s*&?\\[([\\s\\S]*?)\\]`));
  if (!match) {
    return 0;
  }
  return [...match[1].matchAll(/"([^"]+)"/g)].length;
}

function extractFrontendConnectors(source) {
  const plannedArray = source.match(
    /export const plannedConnectors: PlannedConnector\[] = \[([\s\S]*?)\];/,
  );
  if (!plannedArray) {
    throw new Error("Could not find plannedConnectors array in frontend source.");
  }

  const connectors = new Map();
  for (const block of splitTopLevelObjects(plannedArray[1], /\n  \{/g)) {
    const id = readStringField(block, "id");
    if (!id) {
      continue;
    }
    connectors.set(id, {
      id,
      name: readStringField(block, "name"),
      category: readStringField(block, "category"),
      setupPhase: readStringField(block, "setupPhase"),
      configSurfaceCount: countStringArrayField(block, "configSurfaces"),
      automationGateCount: countStringArrayField(block, "automationGates"),
      manualWorkflowCount: countStringArrayField(block, "manualWorkflow"),
    });
  }

  return connectors;
}

function validateConfigCreationPlanContract(source) {
  const errors = [];
  const functionBody = source.match(
    /export function getPlannedConnectorConfigCreationPlan\([\s\S]*?\n}\n\nexport function getPlannedConnectorConfigCreationPlans/,
  )?.[0];

  if (!functionBody) {
    return ["missing getPlannedConnectorConfigCreationPlan export"];
  }

  for (const stepId of [
    "detect",
    "dryRunDiff",
    "backup",
    "apply",
    "verify",
    "rollback",
    "offCleanup",
  ]) {
    if (!functionBody.includes(`id: "${stepId}"`)) {
      errors.push(`config creation plan missing ${stepId} step`);
    }
  }

  if (!functionBody.includes("automationEnabled: false")) {
    errors.push("config creation plan must keep automation disabled by default");
  }
  if (!source.includes("formatPlannedConnectorConfigCreationPlansMarkdown")) {
    errors.push("config creation plans must have copyable markdown formatter");
  }

  return errors;
}

function validateBackendConfigCreationPlanContract(source) {
  const errors = [];
  const constantBody = source.match(
    /const PLANNED_CONFIG_CREATION_STEPS:\s*\[&str;\s*7\]\s*=\s*\[([\s\S]*?)\];/,
  )?.[1];
  if (!constantBody) {
    return ["missing PLANNED_CONFIG_CREATION_STEPS backend contract"];
  }
  for (const label of [
    "Detect config surface",
    "Show dry-run diff",
    "Create backup",
    "Apply with consent",
    "Verify in Doctor",
    "Rollback safely",
    "Clean up in Off mode",
  ]) {
    if (!constantBody.includes(`"${label}"`)) {
      errors.push(`backend config creation plan missing "${label}"`);
    }
  }
  if (!source.includes("config_creation_steps: PLANNED_CONFIG_CREATION_STEPS")) {
    errors.push("planned backend connectors must expose config_creation_steps");
  }
  return errors;
}

function extractBackendConnectors(source) {
  const registry = extractArrayBody(
    source,
    /const PLANNED_CLIENT_SPECS:\s*\[PlannedClientSpec;\s*\d+\]\s*=\s*\[([\s\S]*?)\];/,
    "PLANNED_CLIENT_SPECS registry in Rust source",
  );

  const connectors = new Map();
  for (const block of splitTopLevelObjects(registry, /PlannedClientSpec\s*\{/g)) {
    const id = readStringField(block, "id");
    if (!id) {
      continue;
    }
    connectors.set(id, {
      id,
      name: readStringField(block, "name"),
      category: readStringField(block, "category"),
      setupPhase: readStringField(block, "setup_phase"),
      detectionSourceCount: countStringArrayField(block, "detection_sources"),
      configSurfaceCount: countStringArrayField(block, "config_locations"),
      automationGateCount: countStringArrayField(block, "automation_gates"),
      manualWorkflowCount: countStringArrayField(block, "manual_workflow"),
    });
  }

  return connectors;
}

function difference(left, right) {
  const rightSet = new Set(right);
  return left.filter((item) => !rightSet.has(item));
}

const frontendSource = readFile(frontendPath);
const backendSource = readFile(backendPath);
const frontendConnectors = extractFrontendConnectors(frontendSource);
const backendConnectors = extractBackendConnectors(backendSource);
const frontendIds = uniqueSorted([...frontendConnectors.keys()]);
const backendIds = uniqueSorted([...backendConnectors.keys()]);

const frontendOnly = difference(frontendIds, backendIds);
const backendOnly = difference(backendIds, frontendIds);

if (frontendOnly.length > 0 || backendOnly.length > 0) {
  console.error("Planned connector registries are out of sync.");
  if (frontendOnly.length > 0) {
    console.error(`Only in ${frontendPath}: ${frontendOnly.join(", ")}`);
  }
  if (backendOnly.length > 0) {
    console.error(`Only in ${backendPath}: ${backendOnly.join(", ")}`);
  }
  process.exit(1);
}

if (frontendIds.length === 0) {
  console.error("No planned connectors found; expected at least one registry entry.");
  process.exit(1);
}

const metadataErrors = [];
metadataErrors.push(...validateConfigCreationPlanContract(frontendSource));
metadataErrors.push(...validateBackendConfigCreationPlanContract(backendSource));
for (const id of frontendIds) {
  const frontend = frontendConnectors.get(id);
  const backend = backendConnectors.get(id);
  if (!frontend || !backend) {
    continue;
  }
  for (const field of ["name", "category"]) {
    if (frontend[field] !== backend[field]) {
      metadataErrors.push(`${id}: ${field} mismatch (${frontend[field]} !== ${backend[field]})`);
    }
  }
  const frontendPhase = frontend.setupPhase?.toLowerCase();
  if (frontendPhase !== backend.setupPhase) {
    metadataErrors.push(
      `${id}: setup phase mismatch (${frontend.setupPhase} !== ${backend.setupPhase})`,
    );
  }
  if (backend.detectionSourceCount < 1) {
    metadataErrors.push(`${id}: backend detection sources missing`);
  }
  if (frontend.configSurfaceCount < 3 || backend.configSurfaceCount < 1) {
    metadataErrors.push(`${id}: config surface metadata incomplete`);
  }
  if (frontend.automationGateCount < 3 || backend.automationGateCount < 3) {
    metadataErrors.push(`${id}: automation gate metadata incomplete`);
  }
  if (frontend.manualWorkflowCount < 3 || backend.manualWorkflowCount < 3) {
    metadataErrors.push(`${id}: manual workflow metadata incomplete`);
  }
}

if (metadataErrors.length > 0) {
  console.error("Planned connector metadata is incomplete or out of sync.");
  for (const error of metadataErrors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log(
  `Planned connector registries match with metadata (${frontendIds.length} connectors): ${frontendIds.join(", ")}`,
);
