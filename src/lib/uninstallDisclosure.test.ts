import { describe, expect, it } from "vitest";

import {
  uninstallDisclosureFooter,
  uninstallDisclosureItems,
  uninstallDisclosureTitle,
} from "./uninstallDisclosure";

describe("uninstallDisclosure", () => {
  it("uses Mac AI Switchboard product naming", () => {
    expect(uninstallDisclosureTitle).toBe("Uninstall Mac AI Switchboard?");
    expect(uninstallDisclosureFooter).toContain("Mac AI Switchboard");
    expect(uninstallDisclosureFooter).toContain("Off mode");
  });

  it("lists the reversible local footprint removed by uninstall", () => {
    const allCopy = uninstallDisclosureItems
      .flatMap((item) => [item.text, ...item.paths])
      .join(" ");

    expect(allCopy).toContain("Claude Code");
    expect(allCopy).toContain("Codex");
    expect(allCopy).toContain("~/Library/Application Support/Headroom");
    expect(allCopy).toContain("~/.headroom");
    expect(allCopy).toContain("~/Library/LaunchAgents/");
    expect(allCopy).toContain("Keychain");
    expect(allCopy).toContain("Ponytail");
    expect(allCopy).toContain("backup files");
  });

  it("keeps stable ids for modal rendering", () => {
    expect(uninstallDisclosureItems.map((item) => item.id)).toEqual([
      "client-hooks",
      "managed-hooks",
      "managed-storage",
      "login-item",
      "app-state",
      "plugins-backups",
    ]);
  });
});
