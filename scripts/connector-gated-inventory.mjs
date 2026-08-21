// Keep this list independent from command output so readiness cannot validate
// an observed inventory against itself. Update it alongside connector promotion
// policy when a native-write gate changes.
export const authoritativeGatedNativeWriteConnectors = ["cursor"];

export function compareGatedNativeWriteInventory(observed) {
  const expected = authoritativeGatedNativeWriteConnectors;
  const values = Array.isArray(observed) ? observed : [];
  const duplicates = values.filter((value, index) => values.indexOf(value) !== index);
  const missing = expected.filter((value) => !values.includes(value));
  const extra = values.filter((value) => !expected.includes(value));
  return {
    matches: values.length > 0 && duplicates.length === 0 && missing.length === 0 && extra.length === 0,
    expected: [...expected],
    observed: [...values],
    duplicates: [...new Set(duplicates)],
    missing: [...new Set(missing)],
    extra: [...new Set(extra)],
  };
}
