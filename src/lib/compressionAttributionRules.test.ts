import { describe, expect, it } from "vitest";

import {
  compressionAttributionRules,
  describeCompressionAttributionPolicy,
} from "./compressionAttributionRules";

describe("compressionAttributionRules", () => {
  it("keeps cache hits out of compression family", () => {
    const cacheRule = compressionAttributionRules.find((rule) =>
      rule.source.includes("cache hit"),
    );
    expect(cacheRule?.family).toBe("cache");
    expect(cacheRule?.label).toBe("estimated");
  });

  it("documents the cross-cutting policy", () => {
    expect(describeCompressionAttributionPolicy()).toMatch(/separate from live Headroom compression/i);
  });
});
