export function validateWorldClassFixtures(schema, fixtures) {
  const failures = [];
  if (!Array.isArray(fixtures)) return ["benchmarks/fixtures.json must be an array"];
  if (!Number.isInteger(schema.minimumFixtures) || !Number.isInteger(schema.minimumCategories)) {
    failures.push("benchmark schema minimums must be integers");
  }
  if (fixtures.length < schema.minimumFixtures) failures.push(`expected at least ${schema.minimumFixtures} fixtures, found ${fixtures.length}`);
  const categories = new Set();
  const identities = new Set();
  const requiredFields = schema.requiredFields ?? [];
  for (const fixture of fixtures) {
    const label = fixture?.name ?? "unknown";
    if (!fixture || typeof fixture !== "object" || Array.isArray(fixture)) {
      failures.push(`fixture "${label}" must be an object`);
      continue;
    }
    for (const field of requiredFields) if (!(field in fixture)) failures.push(`fixture "${label}" missing field ${field}`);
    if (typeof fixture.category !== "string" || fixture.category.trim() === "") failures.push(`fixture "${label}" category must be a non-empty string`);
    if (typeof fixture.name !== "string" || fixture.name.trim() === "") failures.push("fixture name must be a non-empty string");
    const identity = `${fixture.category}/${fixture.name}`;
    if (identities.has(identity)) failures.push(`duplicate benchmark fixture identity: ${identity}`);
    identities.add(identity);
    categories.add(fixture.category);
    for (const field of ["original", "optimized"]) if (typeof fixture[field] !== "string") failures.push(`fixture "${label}" ${field} must be a string`);
    if (!Number.isFinite(fixture.latencyOverheadMs) || fixture.latencyOverheadMs < 0) failures.push(`fixture "${label}" latencyOverheadMs must be a finite non-negative number`);
    for (const field of ["relevantFacts", "optimizedFacts", "wrongOmissions"]) {
      if (!Array.isArray(fixture[field]) || fixture[field].some((value) => typeof value !== "string")) failures.push(`fixture "${label}" ${field} must be an array of strings`);
    }
    if (!["pass", "fail", "not_applicable"].includes(fixture.agentSuccessProxy)) failures.push(`fixture "${label}" agentSuccessProxy is invalid`);
    const relevant = Array.isArray(fixture.relevantFacts) ? fixture.relevantFacts : [];
    const optimized = new Set(Array.isArray(fixture.optimizedFacts) ? fixture.optimizedFacts : []);
    const retention = relevant.length === 0 ? 100 : (relevant.filter((fact) => optimized.has(fact)).length / relevant.length) * 100;
    if (retention < schema.qualityGates.minimumRelevantFactRetentionPct) failures.push(`fixture "${label}" retention ${retention}% below gate`);
    const omissions = Array.isArray(fixture.wrongOmissions) ? fixture.wrongOmissions : [];
    const omissionRate = relevant.length === 0 ? 0 : (omissions.length / relevant.length) * 100;
    if (omissionRate > schema.qualityGates.maximumWrongOmissionRatePct) failures.push(`fixture "${label}" wrong omission rate ${omissionRate}% above gate`);
  }
  if (categories.size < schema.minimumCategories) failures.push(`expected at least ${schema.minimumCategories} categories, found ${categories.size}`);
  for (const requiredCategory of schema.requiredCategories ?? []) if (!categories.has(requiredCategory)) failures.push(`missing required category: ${requiredCategory}`);
  return failures;
}
