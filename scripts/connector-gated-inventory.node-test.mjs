import test from "node:test";
import assert from "node:assert/strict";
import { compareGatedNativeWriteInventory } from "./connector-gated-inventory.mjs";

test("accepts the authoritative gated connector inventory", () => {
  assert.equal(compareGatedNativeWriteInventory(["cursor"]).matches, true);
});

test("rejects empty, missing, duplicate, and extra gated connector inventories", () => {
  for (const observed of [[], ["other"], ["cursor", "cursor"], ["cursor", "other"]]) {
    assert.equal(compareGatedNativeWriteInventory(observed).matches, false);
  }
});
