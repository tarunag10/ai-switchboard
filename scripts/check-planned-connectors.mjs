import fs from "node:fs";

const frontendPath = "src/lib/plannedConnectors.ts";
const appPath = "src/App.tsx";
const backendPath = "src-tauri/src/client_adapters.rs";
const cliPath = "scripts/repo-intelligence.mjs";
const repoApiPath = "src-tauri/src/repo_intelligence.rs";
const compatibilityMatrixPath = "research/tool-compatibility-matrix.md";

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

function extractConnectorsFromArray(source, pattern, label) {
  const array = source.match(pattern);
  if (!array) {
    throw new Error(`Could not find ${label} array in frontend source.`);
  }
  const connectors = new Map();
  for (const block of splitTopLevelObjects(array[1], /\n  \{/g)) {
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

function extractFrontendConnectors(source) {
  return extractConnectorsFromArray(
    source,
    /export const plannedConnectors: PlannedConnector\[] = \[([\s\S]*?)\];/,
    "plannedConnectors",
  );
}

function extractManagedFrontendConnectors(source) {
  return extractConnectorsFromArray(
    source,
    /export const managedConnectorDossiers: ManagedConnectorDossier\[] = \[([\s\S]*?)\];/,
    "managedConnectorDossiers",
  );
}

function extractPromotedSidecarConnectorIds(source) {
  const body = extractArrayBody(
    source,
    /export const promotedSidecarConnectorIds = new Set\(\[([\s\S]*?)\]\);/,
    "promotedSidecarConnectorIds",
  );
  return uniqueSorted([...body.matchAll(/"([^"]+)"/g)].map((match) => match[1]));
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
  for (const snippet of [
    "target path",
    "before/after",
    "managed marker boundary",
    "rollback preview",
    "confirmation phrase",
    "Fixture-home restore test",
    "Fixture-home rollback test",
    "Fixture-home Off-mode cleanup",
  ]) {
    if (!functionBody.includes(snippet)) {
      errors.push(`frontend config creation evidence missing "${snippet}"`);
    }
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
  const idBody = source.match(
    /const PLANNED_CONFIG_CREATION_STEP_IDS:\s*\[&str;\s*7\]\s*=\s*\[([\s\S]*?)\];/,
  )?.[1];
  if (!idBody) {
    errors.push("missing PLANNED_CONFIG_CREATION_STEP_IDS backend contract");
  } else {
    for (const id of [
      "detect",
      "dryRunDiff",
      "backup",
      "apply",
      "verify",
      "rollback",
      "offCleanup",
    ]) {
      if (!idBody.includes(`"${id}"`)) {
        errors.push(`backend config creation plan missing "${id}" step id`);
      }
    }
  }
  if (
    !source.includes("config_creation_steps")
    || !source.includes("PLANNED_CONFIG_CREATION_STEPS")
  ) {
    errors.push("planned backend connectors must expose config_creation_steps");
  }
  if (
    !source.includes("config_creation_step_details")
    || !source.includes("planned_config_creation_step_details(spec)")
  ) {
    errors.push("planned backend connectors must expose structured config_creation_step_details");
  }
  if (!source.includes("required_evidence")) {
    errors.push("planned backend config creation steps must expose required_evidence");
  }
  for (const snippet of [
    "target path",
    "before/after",
    "managed marker boundary",
    "rollback preview",
    "confirmation phrase",
    "Fixture-home restore test",
    "Fixture-home rollback test",
    "Fixture-home Off-mode cleanup",
  ]) {
    if (!source.includes(snippet)) {
      errors.push(`backend config creation evidence missing "${snippet}"`);
    }
  }
  for (const snippet of [
    "spec.detection_sources.join",
    "spec.config_locations.join",
    "automation_gates",
    "spec.name",
    "spec.manual_workflow.join",
  ]) {
    if (!source.includes(snippet)) {
      errors.push(`backend config creation details must derive from ${snippet}`);
    }
  }
  for (const [connectorId, snippets] of Object.entries({
    gemini_cli: ["PATH: gemini", "Gemini provider config", "model/account compatibility"],
    opencode: ["PATH: opencode", "OpenCode provider config", "exact previous provider config"],
    cursor: ["PATH: cursor", "Cursor profile", "extension-managed secrets"],
    grok_cli: ["PATH: grok", "PATH: xai", "Doctor guardrails"],
  })) {
    for (const snippet of snippets) {
      if (!source.includes(snippet)) {
        errors.push(`${connectorId}: backend config creation evidence missing "${snippet}"`);
      }
    }
  }
  return errors;
}

function validateCliConnectorDossierContract(source, connectorIds) {
  const errors = [];
  const dossierBody = source.match(
    /const plannedConnectorDossiers = \{([\s\S]*?)\n\};\n\nfunction buildConfigReadiness/,
  )?.[1];

  if (!dossierBody) {
    return ["missing plannedConnectorDossiers CLI handoff contract"];
  }

  for (const id of connectorIds) {
    if (!dossierBody.includes(`${id}: {`)) {
      errors.push(`${id}: CLI connector dossier missing`);
    }
  }

  for (const snippet of [
    "plannedConnectorName",
    "nextGate",
    "safetyDossier",
    "configPathStrategy",
    "accountCaveat",
    "rollbackStrategy",
    "requiredEvidence",
    "dry-run diff artifact",
    "Fixture-home restore test",
    "Fixture-home rollback test",
    "Fixture-home Off-mode cleanup",
  ]) {
    if (!source.includes(snippet)) {
      errors.push(`CLI connector handoff contract missing "${snippet}"`);
    }
  }

  for (const [connectorId, snippets] of Object.entries({
    gemini_cli: ["Detect PATH: gemini", "provider settings"],
    opencode: ["Detect PATH: opencode", "provider-config backup"],
    cursor: ["Cursor app/profile", "profile settings backup"],
    grok_cli: ["PATH: grok", "PATH: xai", "Doctor guardrails"],
    amazon_q: ["AWS credentials", "SSO cache"],
  })) {
    for (const snippet of snippets) {
      if (!dossierBody.includes(snippet)) {
        errors.push(`${connectorId}: CLI connector dossier missing "${snippet}"`);
      }
    }
  }

  if (!source.includes("Planned connector: ${configReadiness.plannedConnectorName}")) {
    errors.push("CLI markdown handoff must include connector name and id");
  }
  if (!source.includes("evidence required: ${step.requiredEvidence.join")) {
    errors.push("CLI markdown handoff must include config gate evidence");
  }

  return errors;
}

function validateRepoApiConnectorDossierContract(source) {
  const errors = [];

  for (const snippet of [
    "RepoAgentConfigReadiness",
    "RepoAgentConfigReadinessDossier",
    "RepoAgentConfigReadinessGate",
    "config_readiness",
    "build_agent_config_readiness",
    "planned_connector_dossier",
    "PLANNED_CONFIG_GATES",
    "dry-run diff artifact",
    "Fixture-home restore test",
    "Fixture-home rollback test",
    "Fixture-home Off-mode cleanup",
  ]) {
    if (!source.includes(snippet)) {
      errors.push(`Repo Intelligence API connector handoff contract missing "${snippet}"`);
    }
  }

  for (const [agentId, snippets] of Object.entries({
    gemini: ["id: \"gemini_cli\"", "Detect PATH: gemini", "provider settings"],
    opencode: ["id: \"opencode\"", "Detect PATH: opencode", "provider-config backup"],
    cursor: ["id: \"cursor\"", "Cursor app/profile", "profile settings backup"],
    grok: ["id: \"grok_cli\"", "PATH: grok", "PATH: xai", "Doctor guardrails"],
    qwen: ["id: \"qwen_code\"", "PATH: qwen-code", "PATH: qwen"],
    amazonq: ["id: \"amazon_q\"", "AWS credentials", "SSO cache"],
    zed: ["id: \"zed_ai\"", "Zed app"],
  })) {
    for (const snippet of snippets) {
      if (!source.includes(snippet)) {
        errors.push(`${agentId}: Repo Intelligence API connector dossier missing "${snippet}"`);
      }
    }
  }

  for (const assertion of [
    "assert!(codex.config_readiness.is_none())",
    "expect(\"gemini config readiness\")",
    "assert_eq!(gemini_readiness.planned_connector_id, \"gemini_cli\")",
    "assert_eq!(gemini_readiness.gated_steps.len(), 7)",
    "expect(\"cursor config readiness\")",
  ]) {
    if (!source.includes(assertion)) {
      errors.push(`Repo Intelligence API connector readiness tests missing "${assertion}"`);
    }
  }

  return errors;
}

function validateCompatibilityMatrixContract(source, connectors) {
  const errors = [];
  for (const connector of connectors.values()) {
    if (!connector.name || !source.includes(`| ${connector.name} |`)) {
      errors.push(`${connector.id}: compatibility matrix missing ${connector.name}`);
    }
  }

  for (const snippet of [
    "Gemini CLI Detection-Only Gate",
    "Detection source: `PATH: gemini`, `~/.gemini`, and `~/.config/gemini`.",
    "dry-run diff, exact backup, apply, verify, rollback, and Off mode cleanup",
    "keep Gemini as `planned` and `guide`; do not convert to managed setup yet",
  ]) {
    if (!source.includes(snippet)) {
      errors.push(`compatibility matrix missing "${snippet}"`);
    }
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
const appSource = readFile(appPath);
const backendSource = readFile(backendPath);
const cliSource = readFile(cliPath);
const repoApiSource = readFile(repoApiPath);
const compatibilityMatrixSource = readFile(compatibilityMatrixPath);
const frontendConnectors = extractFrontendConnectors(frontendSource);
const managedFrontendConnectors = extractManagedFrontendConnectors(frontendSource);
const promotedSidecarIds = extractPromotedSidecarConnectorIds(frontendSource);
const allFrontendConnectors = new Map([
  ...managedFrontendConnectors,
  ...frontendConnectors,
]);
const backendConnectors = extractBackendConnectors(backendSource);
const frontendIds = uniqueSorted([...frontendConnectors.keys()]);
const managedFrontendIds = uniqueSorted([...managedFrontendConnectors.keys()]);
const managedFrontendIdSet = new Set(managedFrontendIds);
const promotedSidecarIdSet = new Set(promotedSidecarIds);
const pendingFrontendIds = frontendIds.filter((id) => !promotedSidecarIdSet.has(id));
const allFrontendIds = uniqueSorted([...allFrontendConnectors.keys()]);
const backendIds = uniqueSorted([...backendConnectors.keys()]);

const frontendOnly = difference(allFrontendIds, backendIds);
const backendOnly = difference(backendIds, allFrontendIds);

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

if (allFrontendIds.length === 0) {
  console.error("No connector metadata found; expected at least one registry entry.");
  process.exit(1);
}

const metadataErrors = [];
metadataErrors.push(...validateConfigCreationPlanContract(frontendSource));
metadataErrors.push(...validateBackendConfigCreationPlanContract(backendSource));
metadataErrors.push(...validateCliConnectorDossierContract(cliSource, allFrontendIds));
metadataErrors.push(...validateRepoApiConnectorDossierContract(repoApiSource));
metadataErrors.push(
  ...validateCompatibilityMatrixContract(
    compatibilityMatrixSource,
    allFrontendConnectors,
  ),
);
if (!appSource.includes("configPlan.steps.map((step) =>")) {
  metadataErrors.push("planned connector UI must render every config creation step");
}
if (!appSource.includes("connector.configCreationStepDetails")) {
  metadataErrors.push("planned connector UI must render structured config creation step details");
}
if (appSource.includes("configPlan.steps.slice(")) {
  metadataErrors.push("planned connector UI must not truncate config creation steps");
}
for (const id of allFrontendIds) {
  const frontend = allFrontendConnectors.get(id);
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
  if (!managedFrontendIdSet.has(id) && frontendPhase !== backend.setupPhase) {
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
  `Connector registries match with metadata (${pendingFrontendIds.length} pending planned, ${managedFrontendIds.length + promotedSidecarIds.length} managed, ${frontendIds.length} retained compatibility dossiers): ${pendingFrontendIds.join(", ") || "none"}`,
);
