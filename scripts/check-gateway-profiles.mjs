#!/usr/bin/env node
/**
 * Static governance gate for the TypeScript gateway registry.
 *
 * The release/check scripts run in a plain Node process, while the registry is
 * intentionally shared with the Vite frontend and is not emitted as a second
 * JSON source of truth. This small parser checks the registry's object-shape
 * and trust-boundary invariants without importing credentials, contacting a
 * gateway, or requiring a TypeScript runtime.
 */
import fs from "node:fs";

const sourcePath = "src/lib/gatewayProfiles.ts";
const source = fs.readFileSync(sourcePath, "utf8");
const profilePattern = /\n  \{\n    id: "([^"]+)",(\s|[\s\S]*?)\n  \},/g;
const profiles = [...source.matchAll(profilePattern)].map((match) => ({
  id: match[1],
  body: match[2],
}));
const failures = [];
const allowedCategories = new Set([
  "local cache",
  "observability",
  "remote gateway",
  "enterprise gateway",
]);
const allowedStates = new Set(["managed", "guided", "detected", "gated", "unsupported"]);
const allowedLifecycleStates = new Set(["available", "guided", "blocked"]);
const lifecycleStageIds = [
  "detect",
  "preview",
  "backup",
  "apply",
  "verify",
  "rollback",
  "offCleanup",
];
const allowedBoundaries = new Set(["local", "remote"]);
const allowedSavingsEvidence = new Set(["estimated", "external", "none"]);
const requiredFields = [
  "name",
  "category",
  "state",
  "trafficBoundary",
  "canSeePromptsAndOutputs",
  "canModifyProviderRouting",
  "needsSecrets",
  "supportedClients",
  "disclosure",
  "privacyCaveat",
  "requiredEvidence",
  "doctorChecks",
  "rollbackGuidance",
  "offModeGuidance",
  "savingsEvidence",
  "setupGuidance",
  "lifecycle",
];

const property = (body, field) =>
  body.match(new RegExp(`\\b${field}:\\s*([^,\\n]+)`))?.[1]?.trim() ?? "";
const stringValue = (body, field) => {
  const match = body.match(new RegExp("\\b" + field + ":\\s*[\"`]([\\s\\S]*?)[\"`]"));
  return match?.[1] ?? "";
};
const arrayBody = (body, field) => body.match(new RegExp(`\\b${field}:\\s*\\[([\\s\\S]*?)\\]`))?.[1] ?? "";
const hasArrayItems = (body, field) => arrayBody(body, field).trim().length > 0;

if (profiles.length === 0) failures.push(`No gateway profiles found in ${sourcePath}`);

const seen = new Set();
for (const profile of profiles) {
  if (seen.has(profile.id)) failures.push(`${profile.id}: duplicate profile id`);
  seen.add(profile.id);

  for (const field of requiredFields) {
    if (!new RegExp(`\\b${field}:`).test(profile.body)) {
      failures.push(`${profile.id}: missing ${field}`);
    }
  }

  const category = stringValue(profile.body, "category");
  const state = stringValue(profile.body, "state");
  const boundary = stringValue(profile.body, "trafficBoundary");
  const savings = stringValue(profile.body, "savingsEvidence");
  const profileState = stringValue(profile.body, "state");
  if (!allowedCategories.has(category)) failures.push(`${profile.id}: invalid category ${category}`);
  if (!allowedStates.has(state)) failures.push(`${profile.id}: invalid state ${state}`);
  if (!allowedBoundaries.has(boundary)) failures.push(`${profile.id}: invalid trafficBoundary ${boundary}`);
  if (!allowedSavingsEvidence.has(savings)) failures.push(`${profile.id}: invalid savingsEvidence ${savings}`);

  for (const field of ["supportedClients", "requiredEvidence", "doctorChecks"]) {
    if (!hasArrayItems(profile.body, field)) failures.push(`${profile.id}: ${field} must not be empty`);
  }

  const disclosure = stringValue(profile.body, "disclosure");
  if (!disclosure) failures.push(`${profile.id}: disclosure must not be empty`);
  for (const field of ["name", "privacyCaveat", "rollbackGuidance", "offModeGuidance", "setupGuidance"]) {
    if (!stringValue(profile.body, field).trim()) {
      failures.push(`${profile.id}: ${field} must not be empty`);
    }
  }
  if (boundary === "remote" && !/remote|gateway|endpoint|trace|export/i.test(disclosure)) {
    failures.push(`${profile.id}: remote profile disclosure is missing the trust boundary`);
  }

  const canModifyRouting = property(profile.body, "canModifyProviderRouting") === "true";
  if (boundary === "local" && canModifyRouting) {
    failures.push(`${profile.id}: local profile cannot modify provider routing`);
  }

  const needsSecrets = property(profile.body, "needsSecrets") === "true";
  const setupGuidance = stringValue(profile.body, "setupGuidance");
  if (needsSecrets && !/secret|token|key|secure|credential/i.test(setupGuidance)) {
    failures.push(`${profile.id}: secret-bearing profile must explain secure secret handling`);
  }

  const lifecycle = profile.body.match(/\blifecycle:\s*\{([\s\S]*?)\n\s*\},/);
  if (!lifecycle) {
    failures.push(`${profile.id}: lifecycle contract is missing`);
  } else {
    const lifecycleBody = lifecycle[1];
    const automation = lifecycleBody.match(/\bautomationEnabled:\s*(true|false)/)?.[1];
    const stageStates = [...lifecycleBody.matchAll(/\bid:\s*"([^"]+)"[\s\S]*?\bstate:\s*"([^"]+)"/g)];
    const stageEvidence = [...lifecycleBody.matchAll(/\bid:\s*"([^"]+)"[\s\S]*?\bevidence:\s*["`]([\s\S]*?)["`]/g)];
    const ids = stageStates.map(([, id]) => id);
    if (ids.length !== lifecycleStageIds.length || ids.some((id, index) => id !== lifecycleStageIds[index])) {
      failures.push(`${profile.id}: lifecycle stages must follow ${lifecycleStageIds.join(", ")} order`);
    }
    for (const [, id, state] of stageStates) {
      if (!allowedLifecycleStates.has(state)) failures.push(`${profile.id}: invalid lifecycle state ${state} for ${id}`);
    }
    if (stageEvidence.length !== lifecycleStageIds.length || stageEvidence.some(([, , evidence]) => !evidence.trim())) {
      failures.push(`${profile.id}: every lifecycle stage must include non-empty evidence`);
    }
    const hasBlockedStage = stageStates.some(([, , state]) => state !== "available");
    if (automation === "true" && hasBlockedStage) {
      failures.push(`${profile.id}: automationEnabled requires all lifecycle stages to be available`);
    }
    if (profileState === "managed" && automation !== "true") {
      failures.push(`${profile.id}: managed profiles must enable lifecycle automation`);
    }
    if (profileState !== "managed" && automation === "true") {
      failures.push(`${profile.id}: non-managed profiles may not enable lifecycle automation`);
    }
  }
}

if (failures.length) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log(`Gateway profile governance validated (${profiles.length} profiles).`);
