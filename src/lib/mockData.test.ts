import { describe, expect, it } from "vitest";
import { mockDashboard } from "./mockData";

describe("mock dashboard seed", () => {
  it("includes required headroom tool metadata", () => {
    const headroom = mockDashboard.tools.find((tool) => tool.id === "headroom");

    expect(headroom).toBeDefined();
    expect(headroom?.required).toBe(true);
    expect(headroom?.runtime).toBe("python");
  });
});
