import { describe, expect, it } from "vitest";

import {
  LEGACY_SWITCHBOARD_ROUTING_FILE,
  SWITCHBOARD_ROUTING_FILE,
  switchboardDryRunBackupPath,
  switchboardManagedMarkerId,
  switchboardRoutingPath,
  SwitchboardIdentitySlug,
} from "./switchboardIdentity";

describe("switchboardIdentity", () => {
  it("uses the ai-switchboard slug for managed markers and routing files", () => {
    expect(SwitchboardIdentitySlug.AiSwitchboard).toBe("ai-switchboard");
    expect(SWITCHBOARD_ROUTING_FILE).toBe("ai-switchboard-routing.md");
    expect(LEGACY_SWITCHBOARD_ROUTING_FILE).toBe(
      "mac-ai-switchboard-routing.md",
    );
    expect(switchboardManagedMarkerId("continue")).toBe(
      "ai-switchboard:continue",
    );
    expect(switchboardRoutingPath("~/.continue")).toBe(
      "~/.continue/ai-switchboard-routing.md",
    );
    expect(switchboardDryRunBackupPath("~/.continue/config.yaml")).toBe(
      "~/.continue/config.yaml.ai-switchboard.bak",
    );
  });
});
