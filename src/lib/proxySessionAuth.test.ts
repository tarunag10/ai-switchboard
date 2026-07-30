import { describe, expect, it } from "vitest";

import { describeProxySessionAuthStatus } from "./proxySessionAuth";

describe("describeProxySessionAuthStatus", () => {
  it("labels enforced mode", () => {
    const result = describeProxySessionAuthStatus({
      available: true,
      enforce: true,
      fingerprint: "abcd1234…",
      status: "session_token_enforced",
      detail: "Loopback proxy requires the session header.",
      validatedRequestCount: 2,
      rejectedRequestCount: 0,
    });
    expect(result.label).toBe("Enforced");
  });

  it("warns when token is only available", () => {
    const result = describeProxySessionAuthStatus({
      available: true,
      enforce: false,
      fingerprint: "abcd1234…",
      status: "session_token_available",
      detail: "Session token is available.",
      validatedRequestCount: 0,
      rejectedRequestCount: 0,
    });
    expect(result.tone).toBe("warning");
  });
});
