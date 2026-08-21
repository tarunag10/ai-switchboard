import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { validateConnectorPromotionFixture } from "./connector-promotion-contract.mjs";

const fixture = JSON.parse(
  fs.readFileSync(path.join(process.cwd(), "fixtures/connector-promotion-evidence.json"), "utf8"),
);

test("accepts the canonical connector promotion fixture", () => {
  assert.deepEqual(validateConnectorPromotionFixture(fixture), []);
});

test("rejects duplicate, overlapping, malformed, and reordered promotion data", () => {
  const duplicate = { ...fixture, promotedNativeConnectorIds: ["goose", "goose"] };
  assert.match(validateConnectorPromotionFixture(duplicate).join("\n"), /duplicate IDs/);

  const overlap = { ...fixture, gatedNativeConnectorIds: ["cursor", "goose"] };
  assert.match(validateConnectorPromotionFixture(overlap).join("\n"), /overlap/);

  const malformed = { ...fixture, promotedNativeConnectorIds: ["Goose"] };
  assert.match(validateConnectorPromotionFixture(malformed).join("\n"), /lowercase/);

  const reordered = { ...fixture, requiredSidecarStages: [...fixture.requiredSidecarStages].reverse() };
  assert.match(validateConnectorPromotionFixture(reordered).join("\n"), /canonical lifecycle order/);
});
