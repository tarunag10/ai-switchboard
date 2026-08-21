import { describe, expect, it } from "vitest";

import { isCurrentConnectorRefresh } from "./connectorRefresh";

describe("connector refresh generations", () => {
  it("rejects an older response after a newer refresh starts", () => {
    expect(isCurrentConnectorRefresh(1, 2)).toBe(false);
    expect(isCurrentConnectorRefresh(2, 2)).toBe(true);
  });

  it("does not treat a future generation as current", () => {
    expect(isCurrentConnectorRefresh(3, 2)).toBe(false);
  });
});
