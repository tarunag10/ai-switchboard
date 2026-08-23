import { describe, expect, it } from "vitest";

import { addonCopy } from "./trayAddonCopy";

describe("MarkItDown tray copy", () => {
  it("discloses the managed runtime, hook, permission, and local cache", () => {
    expect(addonCopy.markitdown.whatItDoes).toMatch(/managed Python runtime/);
    expect(addonCopy.markitdown.whatItDoes).toMatch(/Read hook/);
    expect(addonCopy.markitdown.whatItDoes).toMatch(/Switchboard-owned Claude permission/);
    expect(addonCopy.markitdown.whatItDoes).toContain("/tmp/headroom-markitdown");
  });

  it("explains cleanup on disable and uninstall", () => {
    expect(addonCopy.markitdown.disabling).toMatch(/managed Read hook, permission, and local conversion cache/);
    expect(addonCopy.markitdown.uninstalling).toMatch(/permission/);
    expect(addonCopy.markitdown.uninstalling).toMatch(/local conversion cache/);
  });
});
