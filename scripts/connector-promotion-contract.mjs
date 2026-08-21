const REQUIRED_SIDECAR_STAGES = [
  "detected",
  "manualGuide",
  "backupImplemented",
  "applyImplemented",
  "verifyImplemented",
  "rollbackImplemented",
  "offCleanupImplemented",
];

function duplicateValues(values) {
  return [...new Set(values.filter((value, index) => values.indexOf(value) !== index))];
}

function validateIdList(value, label) {
  const errors = [];
  if (!Array.isArray(value) || value.length === 0) {
    errors.push(`${label} must be a non-empty array`);
    return errors;
  }
  if (value.some((id) => typeof id !== "string" || !/^[a-z0-9_]+$/.test(id))) {
    errors.push(`${label} must contain lowercase connector IDs only`);
  }
  const duplicates = duplicateValues(value);
  if (duplicates.length > 0) errors.push(`${label} contains duplicate IDs: ${duplicates.join(", ")}`);
  return errors;
}

export function validateConnectorPromotionFixture(fixture) {
  const errors = [];
  if (!fixture || typeof fixture !== "object") return ["fixture must be an object"];
  if (fixture.schemaVersion !== 1) errors.push("schemaVersion must be 1");
  if (!Array.isArray(fixture.requiredSidecarStages)) {
    errors.push("requiredSidecarStages must be an array");
  } else {
    const duplicates = duplicateValues(fixture.requiredSidecarStages);
    if (duplicates.length > 0) errors.push(`requiredSidecarStages contains duplicates: ${duplicates.join(", ")}`);
    if (JSON.stringify(fixture.requiredSidecarStages) !== JSON.stringify(REQUIRED_SIDECAR_STAGES)) {
      errors.push("requiredSidecarStages must match the canonical lifecycle order");
    }
  }
  errors.push(...validateIdList(fixture.promotedNativeConnectorIds, "promotedNativeConnectorIds"));
  errors.push(...validateIdList(fixture.gatedNativeConnectorIds, "gatedNativeConnectorIds"));
  const promoted = new Set(fixture.promotedNativeConnectorIds ?? []);
  const gated = new Set(fixture.gatedNativeConnectorIds ?? []);
  const overlap = [...promoted].filter((id) => gated.has(id));
  if (overlap.length > 0) errors.push(`promoted and gated connector IDs overlap: ${overlap.join(", ")}`);
  if (!gated.has("cursor")) errors.push("gatedNativeConnectorIds must keep cursor native writes gated");
  if (promoted.has("cursor")) errors.push("cursor cannot be promoted while native writes remain gated");
  return errors;
}

export { REQUIRED_SIDECAR_STAGES };
