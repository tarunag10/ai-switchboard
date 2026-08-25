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
  if (JSON.stringify(value) !== JSON.stringify([...value].sort())) {
    errors.push(`${label} must use canonical sorted connector ID order`);
  }
  return errors;
}

function validatePromotionPartition(
  promotedValue,
  gatedValue,
  expectedConnectorIds,
  label,
) {
  const errors = [];
  const promoted = new Set(Array.isArray(promotedValue) ? promotedValue : []);
  const gated = new Set(Array.isArray(gatedValue) ? gatedValue : []);
  const overlap = [...promoted].filter((id) => gated.has(id));
  if (overlap.length > 0) {
    errors.push(`${label} promoted and gated connector IDs overlap: ${overlap.join(", ")}`);
  }

  if (expectedConnectorIds) {
    const expected = new Set(expectedConnectorIds);
    const classified = new Set([...promoted, ...gated]);
    const unknown = [...classified].filter((id) => !expected.has(id)).sort();
    const missing = [...expected].filter((id) => !classified.has(id)).sort();
    if (unknown.length > 0) {
      errors.push(`${label} contains unknown expansion connector IDs: ${unknown.join(", ")}`);
    }
    if (missing.length > 0) {
      errors.push(`${label} is missing expansion connector IDs: ${missing.join(", ")}`);
    }
  }

  return errors;
}

export function validateConnectorPromotionFixture(
  fixture,
  expectedExpansionConnectorIds,
) {
  const errors = [];
  if (!fixture || typeof fixture !== "object") return ["fixture must be an object"];
  if (fixture.schemaVersion !== 1) errors.push("schemaVersion must be 1");
  if (Object.hasOwn(fixture, "gatedNativeConnectorIds")) {
    errors.push(
      "gatedNativeConnectorIds is unsupported; use gatedNativeConfigConnectorIds",
    );
  }
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
  errors.push(
    ...validateIdList(
      fixture.gatedNativeConfigConnectorIds,
      "gatedNativeConfigConnectorIds",
    ),
  );
  errors.push(
    ...validatePromotionPartition(
      fixture.promotedNativeConnectorIds,
      fixture.gatedNativeConfigConnectorIds,
      expectedExpansionConnectorIds,
      "promotion fixture",
    ),
  );
  const promoted = new Set(fixture.promotedNativeConnectorIds ?? []);
  const gated = new Set(fixture.gatedNativeConfigConnectorIds ?? []);
  if (!gated.has("cursor")) errors.push("gatedNativeConfigConnectorIds must keep cursor native writes gated");
  if (promoted.has("cursor")) errors.push("cursor cannot be promoted while native writes remain gated");
  return errors;
}

function extractArrayIds(source, pattern, label) {
  const match = source.match(pattern);
  if (!match) throw new Error(`missing ${label} declaration`);
  return [...match[1].matchAll(/\bid:\s*"([a-z0-9_]+)"/g)].map(
    (item) => item[1],
  );
}

function extractSetIds(source, exportName) {
  const match = source.match(
    new RegExp(
      `export const ${exportName} = new Set\\(\\[([\\s\\S]*?)\\]\\);`,
    ),
  );
  if (!match) throw new Error(`missing ${exportName} declaration`);
  return [...match[1].matchAll(/"([a-z0-9_]+)"/g)].map((item) => item[1]);
}

export function extractConnectorPromotionFrontendContract(source) {
  const planned = extractArrayIds(
    source,
    /export const plannedConnectors: PlannedConnector\[] = \[([\s\S]*?)\n\];/,
    "plannedConnectors",
  );
  const managed = extractArrayIds(
    source,
    /export const managedConnectorDossiers: ManagedConnectorDossier\[] = \[([\s\S]*?)\n\];/,
    "managedConnectorDossiers",
  );
  return {
    expansionConnectorIds: [...new Set([...managed, ...planned])].sort(),
    promotedNativeConnectorIds: extractSetIds(
      source,
      "promotedNativeConfigConnectorIds",
    ),
    gatedNativeConfigConnectorIds: extractSetIds(
      source,
      "gatedNativeConfigConnectorIds",
    ),
  };
}

export function validateConnectorPromotionConsistency(fixture, frontend) {
  const errors = validateConnectorPromotionFixture(
    fixture,
    frontend.expansionConnectorIds,
  );
  errors.push(
    ...validateIdList(
      frontend.expansionConnectorIds,
      "frontend expansionConnectorIds",
    ),
    ...validateIdList(
      frontend.promotedNativeConnectorIds,
      "frontend promotedNativeConfigConnectorIds",
    ),
    ...validateIdList(
      frontend.gatedNativeConfigConnectorIds,
      "frontend gatedNativeConfigConnectorIds",
    ),
    ...validatePromotionPartition(
      frontend.promotedNativeConnectorIds,
      frontend.gatedNativeConfigConnectorIds,
      frontend.expansionConnectorIds,
      "frontend promotion contract",
    ),
  );
  if (
    JSON.stringify(frontend.promotedNativeConnectorIds) !==
    JSON.stringify(fixture.promotedNativeConnectorIds)
  ) {
    errors.push("frontend promoted native connector IDs do not match the fixture");
  }
  if (
    JSON.stringify(frontend.gatedNativeConfigConnectorIds) !==
    JSON.stringify(fixture.gatedNativeConfigConnectorIds)
  ) {
    errors.push("frontend gated native config connector IDs do not match the fixture");
  }
  return errors;
}

export { REQUIRED_SIDECAR_STAGES };
