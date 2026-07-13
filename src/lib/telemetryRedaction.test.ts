import { describe, expect, it } from "vitest";

import {
  redactTelemetryText,
  sanitizeSentryEvent,
  safeTelemetryContext,
  safeTelemetryError,
} from "./telemetryRedaction";

describe("telemetry redaction", () => {
  it("scrubs common secrets and control characters", () => {
    const value = redactTelemetryText(
      "Authorization: Bearer abcdefghijklmnop sk-ant-test ghp_abcdef\nprivate AuthKey.p8"
    );

    expect(value).not.toContain("abcdefghijklmnop");
    expect(value).not.toContain("sk-ant-test");
    expect(value).not.toContain("ghp_abcdef");
    expect(value).not.toContain("AuthKey.p8");
    expect(value).not.toContain("\n");
    expect(value).toContain("[REDACTED]");
  });

  it("bounds diagnostic context", () => {
    const value = safeTelemetryContext("x".repeat(1000), "missing");
    expect(value.length).toBeLessThanOrEqual(160);
    expect(value.endsWith("…")).toBe(true);
  });

  it("creates a category-only error for remote capture", () => {
    const error = safeTelemetryError("app_update_install_failed");
    expect(error.name).toBe("SwitchboardTelemetryError");
    expect(error.message).toBe("app_update_install_failed");
  });

  it("sanitizes ErrorBoundary events before Sentry receives them", () => {
    const event = sanitizeSentryEvent({
      message: "prompt text should not leave the app",
      logentry: { message: "raw details", params: ["raw"] },
      exception: {
        values: [{ type: "Error", value: "raw prompt", stacktrace: { frames: [] } }],
      },
      request: { data: "request body" },
      user: { email: "user@example.com" },
      breadcrumbs: [{ message: "raw breadcrumb" }],
      extra: { payload: "raw" },
    });

    expect(event.message).toBe("sentry_error");
    expect(event.logentry?.message).toBe("sentry_error");
    expect(event.logentry?.params).toBeUndefined();
    expect(event.exception?.values?.[0]).toEqual({
      type: "Error",
      value: "sentry_error:Error",
      stacktrace: undefined,
    });
    expect(event.request).toBeUndefined();
    expect(event.user).toBeUndefined();
    expect(event.breadcrumbs).toBeUndefined();
    expect(event.extra).toBeUndefined();
  });
});
