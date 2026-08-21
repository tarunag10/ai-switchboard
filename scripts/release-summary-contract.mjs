import { validateReleaseEvidenceTimestamp } from "./release-evidence-time.mjs";

export function extractGeneratedAt(markdown) {
  if (typeof markdown !== "string") return null;
  const line = markdown
    .split("\n")
    .find((candidate) => candidate.startsWith("Generated: "));
  return line ? line.slice("Generated: ".length).trim() || null : null;
}

export function validateSummaryGeneratedAt(
  generatedAt,
  { present = false, passed = false, now = Date.now(), label = "generatedAt" } = {},
) {
  if (generatedAt === null || generatedAt === undefined) {
    return present || passed
      ? { ok: false, reason: `${label} is required for present or passed evidence` }
      : { ok: true };
  }
  if (typeof generatedAt !== "string") {
    return { ok: false, reason: `${label} must be a string or null` };
  }
  return validateReleaseEvidenceTimestamp(generatedAt, { now, label });
}

export function assessSummaryFreshness(
  { summaryGeneratedAt = null, jsonGeneratedAt = null, present = false, passed = false } = {},
  { now = Date.now(), label = "generatedAt" } = {},
) {
  const timestamps = [jsonGeneratedAt, summaryGeneratedAt].filter(
    (value) => value !== null && value !== undefined,
  );
  if (timestamps.length === 0) {
    return {
      fresh: !(present || passed),
      generatedAt: null,
      reason: present || passed ? `${label} is required for present or passed evidence` : null,
    };
  }
  const checks = timestamps.map((generatedAt) =>
    validateSummaryGeneratedAt(generatedAt, { present: true, passed: true, now, label }),
  );
  const failed = checks.find((check) => !check.ok);
  if (failed) return { fresh: false, generatedAt: timestamps[0], reason: failed.reason };
  if (new Set(timestamps).size > 1) {
    return { fresh: false, generatedAt: timestamps[0], reason: `${label} differs between JSON and Markdown evidence` };
  }
  return { fresh: true, generatedAt: timestamps[0], ageMs: checks[0].ageMs, reason: null };
}
