/**
 * Keep optional remote diagnostics metadata-only. This helper is deliberately
 * conservative: Sentry receives a category rather than the original error
 * text, while bounded context values are scrubbed before they are attached.
 */
const MAX_TELEMETRY_TEXT_LENGTH = 160;

const SECRET_PATTERNS: RegExp[] = [
  /sk-ant-[A-Za-z0-9_-]+/gi,
  /sk-proj-[A-Za-z0-9_-]+/gi,
  /ghp_[A-Za-z0-9]+/gi,
  /github_pat_[A-Za-z0-9_]+/gi,
  /(?:authorization\s*:\s*bearer|bearer)\s+[A-Za-z0-9._~+/=-]{8,}/gi,
  /(?:AWS_SECRET_ACCESS_KEY|ANTHROPIC_API_KEY|OPENAI_API_KEY)\s*[=:]\s*[^\s,;]+/gi,
  /BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY/gi,
  /\b[^\s/]+\.(?:p8|pem|p12)\b/gi,
];

export function redactTelemetryText(value: string, fallback = "unspecified"): string {
  const normalized = value.replace(/[\u0000-\u001f\u007f]/g, " ").trim();
  if (!normalized) {
    return fallback;
  }

  const redacted = SECRET_PATTERNS.reduce(
    (text, pattern) => text.replace(pattern, "[REDACTED]"),
    normalized
  );
  const compact = redacted.replace(/\s+/g, " ");
  return compact.length > MAX_TELEMETRY_TEXT_LENGTH
    ? `${compact.slice(0, MAX_TELEMETRY_TEXT_LENGTH - 1)}…`
    : compact;
}

export function safeTelemetryError(category: string): Error {
  const error = new Error(redactTelemetryText(category, "telemetry_error"));
  error.name = "SwitchboardTelemetryError";
  return error;
}

export function safeTelemetryContext(value: string | undefined, fallback: string): string {
  return redactTelemetryText(value ?? "", fallback);
}

export interface SentryEventLike {
  message?: string;
  logentry?: { message?: string; params?: unknown[] };
  exception?: {
    values?: Array<{ type?: string; value?: string; stacktrace?: unknown }>;
  };
  request?: unknown;
  user?: unknown;
  breadcrumbs?: unknown[];
  extra?: unknown;
}

/** Remove unbounded browser context before an event can leave the app. */
export function sanitizeSentryEvent<T extends SentryEventLike>(event: T): T {
  if (event.message) {
    event.message = "sentry_error";
  }
  if (event.logentry?.message) {
    event.logentry.message = "sentry_error";
    event.logentry.params = undefined;
  }
  for (const exception of event.exception?.values ?? []) {
    exception.value = `sentry_error:${redactTelemetryText(exception.type ?? "unknown", "unknown")}`;
    exception.stacktrace = undefined;
  }
  event.request = undefined;
  event.user = undefined;
  event.breadcrumbs = undefined;
  event.extra = undefined;
  return event;
}
