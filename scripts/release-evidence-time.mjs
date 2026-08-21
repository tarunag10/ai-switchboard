export const RELEASE_REPORT_MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000;
export const RELEASE_REPORT_FUTURE_SKEW_MS = 5 * 60 * 1000;

export function validateReleaseEvidenceTimestamp(
  value,
  {
    now = Date.now(),
    maxAgeMs = RELEASE_REPORT_MAX_AGE_MS,
    futureSkewMs = RELEASE_REPORT_FUTURE_SKEW_MS,
    label = "generatedAt",
  } = {},
) {
  if (typeof value !== "string" || value.length === 0) {
    return { ok: false, reason: `${label} must be a non-empty string` };
  }

  const parsed = Date.parse(value);
  if (!Number.isFinite(parsed)) {
    return { ok: false, reason: `${label} must be a valid ISO-8601 timestamp` };
  }
  if (!value.includes("T") || !/[zZ]|[+-]\d{2}:?\d{2}$/.test(value)) {
    return { ok: false, reason: `${label} must include an explicit timezone` };
  }
  if (!Number.isFinite(now) || !Number.isFinite(maxAgeMs) || maxAgeMs < 0) {
    return { ok: false, reason: "timestamp validation options are invalid" };
  }
  if (!Number.isFinite(futureSkewMs) || futureSkewMs < 0) {
    return { ok: false, reason: "futureSkewMs must be a non-negative number" };
  }

  const ageMs = now - parsed;
  if (ageMs < -futureSkewMs) {
    return { ok: false, reason: `${label} is too far in the future` };
  }
  if (ageMs > maxAgeMs) {
    return { ok: false, reason: `${label} is stale` };
  }

  return { ok: true, parsed, ageMs };
}
