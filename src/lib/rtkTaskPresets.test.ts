import { describe, expect, it } from "vitest";

import {
  buildRtkShellProfileSnippet,
  getRtkTaskPreset,
  RTK_TASK_PRESETS,
} from "./rtkTaskPresets";

describe("rtkTaskPresets", () => {
  it.each([
    ["test", "export RTK_TASK_PRESET=test"],
    ["build", "export RTK_TASK_PRESET=build"],
    ["grep", "export RTK_TASK_PRESET=grep"],
    ["git-log", "export RTK_TASK_PRESET=git-log"],
  ])("returns the %s preset and shell snippet", (id, expected) => {
    expect(getRtkTaskPreset(id)?.id).toBe(id);
    expect(buildRtkShellProfileSnippet(id)).toContain(expected);
  });

  it("returns safe absence for unknown preset IDs", () => {
    expect(getRtkTaskPreset("unknown")).toBeUndefined();
    expect(buildRtkShellProfileSnippet("unknown")).toBeNull();
    expect(RTK_TASK_PRESETS).toHaveLength(4);
  });
});
