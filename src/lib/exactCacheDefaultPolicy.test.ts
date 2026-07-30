import { describe, expect, it } from "vitest";

import { recommendExactCacheDefault } from "./exactCacheDefaultPolicy";

describe("recommendExactCacheDefault", () => {
  it("recommends exact cache in full mode when disabled and proxy is reachable", () => {
    const result = recommendExactCacheDefault({
      mode: "full",
      semanticCacheEnabled: false,
      proxyReachable: true,
    });
    expect(result.recommend).toBe(true);
  });

  it("does not recommend in off or rtk-only modes", () => {
    expect(
      recommendExactCacheDefault({
        mode: "off",
        semanticCacheEnabled: false,
        proxyReachable: true,
      }).recommend,
    ).toBe(false);
    expect(
      recommendExactCacheDefault({
        mode: "rtk",
        semanticCacheEnabled: false,
        proxyReachable: true,
      }).recommend,
    ).toBe(false);
  });

  it("does not recommend when cache is already enabled", () => {
    expect(
      recommendExactCacheDefault({
        mode: "full",
        semanticCacheEnabled: true,
        proxyReachable: true,
      }).recommend,
    ).toBe(false);
  });
});
