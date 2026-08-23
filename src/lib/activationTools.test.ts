import { describe, expect, it } from "vitest";
import {
  SELECTIVE_ACTIVATION_LIMIT,
  SELECTIVE_ACTIVATION_TOOLS,
  normalizeActivationSelection,
  validateActivationSelection,
} from "./activationTools";

describe("selective activation catalog", () => {
  it("defines ten unique selectable tools", () => {
    expect(SELECTIVE_ACTIVATION_TOOLS).toHaveLength(10);
    expect(new Set(SELECTIVE_ACTIVATION_TOOLS.map((tool) => tool.id)).size).toBe(10);
  });

  it("requires exactly five known unique tools", () => {
    const five = SELECTIVE_ACTIVATION_TOOLS.slice(0, SELECTIVE_ACTIVATION_LIMIT).map((tool) => tool.id);
    expect(validateActivationSelection(five)).toBeNull();
    expect(validateActivationSelection(five.slice(0, 4))).toMatch(/exactly five/);
    expect(validateActivationSelection([...five, "leanctx"])).toMatch(/exactly five/);
    expect(validateActivationSelection([five[0], five[0], ...five.slice(1, 4)])).toMatch(/duplicate/);
  });

  it("drops malformed persisted values without inventing tools", () => {
    expect(normalizeActivationSelection(["rtk", "rtk", "unknown", 4])).toEqual(["rtk"]);
  });
});
