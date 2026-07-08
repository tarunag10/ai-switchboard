import { afterEach, describe, expect, it, vi } from "vitest";

async function loadSupportUrls() {
  vi.resetModules();
  return import("./supportUrls");
}

describe("supportUrls", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("uses defaults when support env vars are empty", async () => {
    vi.stubEnv("VITE_HEADROOM_SALES_CONTACT_URL", "");
    vi.stubEnv("VITE_HEADROOM_CONTACT_FORM_URL", "");

    const { CONTACT_FORM_URL, SALES_CONTACT_URL, SUPPORT_ISSUES_URL } =
      await loadSupportUrls();

    expect(SALES_CONTACT_URL).toBe("mailto:hello@example.com");
    expect(CONTACT_FORM_URL).toBe("");
    expect(SUPPORT_ISSUES_URL).toBe(
      "https://github.com/tarunag10/mac-ai-switchboard/issues",
    );
  });

  it("trims configured support URLs", async () => {
    vi.stubEnv(
      "VITE_HEADROOM_SALES_CONTACT_URL",
      " https://example.com/sales ",
    );
    vi.stubEnv(
      "VITE_HEADROOM_CONTACT_FORM_URL",
      " https://example.com/contact ",
    );

    const { CONTACT_FORM_URL, SALES_CONTACT_URL } = await loadSupportUrls();

    expect(SALES_CONTACT_URL).toBe("https://example.com/sales");
    expect(CONTACT_FORM_URL).toBe("https://example.com/contact");
  });
});
