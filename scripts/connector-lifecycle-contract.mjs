export const canonicalLifecycleStages = ["detect", "preview", "backup", "apply", "verify", "rollback", "off"];
export const runtimeStageByFixtureStage = {
  detect: "detect",
  preview: "dryRunDiff",
  backup: "backup",
  apply: "apply",
  verify: "verify",
  rollback: "rollback",
  off: "offCleanup",
};

export function validateLifecycleSchema(manifest, fixtures) {
  const failures = [];
  if (!Array.isArray(manifest)) failures.push("connector manifest must be an array");
  if (!fixtures || !Array.isArray(fixtures.connectors)) failures.push("lifecycle fixtures must contain a connectors array");
  if (!Array.isArray(fixtures?.requiredStages) || new Set(fixtures.requiredStages).size !== fixtures.requiredStages.length) {
    failures.push("requiredStages must be a unique array");
  } else if (JSON.stringify(fixtures.requiredStages) !== JSON.stringify(canonicalLifecycleStages)) {
    failures.push(`requiredStages must equal ${canonicalLifecycleStages.join(",")}`);
  }
  if (JSON.stringify(Object.keys(runtimeStageByFixtureStage)) !== JSON.stringify(canonicalLifecycleStages)) {
    failures.push("runtime lifecycle stage mapping must cover every fixture stage exactly once");
  }
  if (!Array.isArray(manifest) || !Array.isArray(fixtures?.connectors)) return failures;

  const manifestIds = new Set();
  for (const connector of manifest) {
    if (!connector || typeof connector.id !== "string" || connector.id.trim() === "") {
      failures.push("manifest connector IDs must be non-empty strings");
      continue;
    }
    if (manifestIds.has(connector.id)) failures.push(`duplicate manifest connector ID: ${connector.id}`);
    manifestIds.add(connector.id);
  }
  const fixtureIds = new Set();
  for (const fixture of fixtures.connectors) {
    if (!fixture || typeof fixture.id !== "string" || fixture.id.trim() === "") {
      failures.push("fixture connector IDs must be non-empty strings");
      continue;
    }
    if (fixtureIds.has(fixture.id)) failures.push(`duplicate lifecycle fixture ID: ${fixture.id}`);
    fixtureIds.add(fixture.id);
    if (!fixture.stages || typeof fixture.stages !== "object" || Array.isArray(fixture.stages)) {
      failures.push(`${fixture.id}: stages must be an object`);
      continue;
    }
    for (const stage of Object.keys(fixture.stages)) {
      if (!canonicalLifecycleStages.includes(stage)) failures.push(`${fixture.id}: unknown lifecycle stage ${stage}`);
    }
    for (const stage of canonicalLifecycleStages) {
      const value = fixture.stages[stage];
      if (value !== null && typeof value !== "string") failures.push(`${fixture.id}: ${stage} must be a string or explicit null`);
    }
  }
  return failures;
}
