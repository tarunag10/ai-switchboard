import { describe, expect, it } from "vitest";

import { describeCursorNativeGate } from "./cursorNativeGate";

describe("describeCursorNativeGate", () => {
  it("keeps native writes blocked when schema is not supported", () => {
    const result = describeCursorNativeGate({
      schemaId: "cursor-native-provider-schema",
      supported: false,
      reason: "Cursor does not document a stable on-disk provider schema.",
      docsUrl: "https://cursor.com/help/models-and-usage/api-keys",
      surfacesDetected: 2,
      evidence: ["remain blocked"],
    });
    expect(result.nativeWritesAllowed).toBe(false);
    expect(result.sidecarAllowed).toBe(true);
  });
});
