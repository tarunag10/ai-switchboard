import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import {
  extractConnectorPromotionFrontendContract,
  validateConnectorPromotionConsistency,
  validateConnectorPromotionFixture,
} from "./connector-promotion-contract.mjs";

const fixture = JSON.parse(
  fs.readFileSync(path.join(process.cwd(), "fixtures/connector-promotion-evidence.json"), "utf8"),
);
const frontend = extractConnectorPromotionFrontendContract(
  fs.readFileSync(path.join(process.cwd(), "src/lib/plannedConnectors.ts"), "utf8"),
);

test("accepts the canonical connector promotion fixture", () => {
  assert.deepEqual(
    validateConnectorPromotionFixture(fixture, frontend.expansionConnectorIds),
    [],
  );
  assert.deepEqual(validateConnectorPromotionConsistency(fixture, frontend), []);
  assert.deepEqual(frontend.expansionConnectorIds, [
    "aider",
    "amazon_q",
    "continue",
    "cursor",
    "gemini_cli",
    "goose",
    "grok_cli",
    "opencode",
    "qwen_code",
    "windsurf",
    "zed_ai",
  ]);
  assert.deepEqual(frontend.gatedNativeConfigConnectorIds, [
    "amazon_q",
    "cursor",
    "qwen_code",
  ]);
});

test("rejects duplicate, overlapping, malformed, and reordered promotion data", () => {
  const duplicate = { ...fixture, promotedNativeConnectorIds: ["goose", "goose"] };
  assert.match(validateConnectorPromotionFixture(duplicate).join("\n"), /duplicate IDs/);

  const overlap = {
    ...fixture,
    gatedNativeConfigConnectorIds: ["amazon_q", "cursor", "goose", "qwen_code"],
  };
  assert.match(validateConnectorPromotionFixture(overlap).join("\n"), /overlap/);

  const malformed = { ...fixture, promotedNativeConnectorIds: ["Goose"] };
  assert.match(validateConnectorPromotionFixture(malformed).join("\n"), /lowercase/);

  const reordered = { ...fixture, requiredSidecarStages: [...fixture.requiredSidecarStages].reverse() };
  assert.match(validateConnectorPromotionFixture(reordered).join("\n"), /canonical lifecycle order/);

  const reorderedIds = { ...fixture, promotedNativeConnectorIds: [...fixture.promotedNativeConnectorIds].reverse() };
  assert.match(validateConnectorPromotionFixture(reorderedIds).join("\n"), /canonical sorted connector ID order/);

  const reorderedGated = {
    ...fixture,
    gatedNativeConfigConnectorIds: [...fixture.gatedNativeConfigConnectorIds].reverse(),
  };
  assert.match(
    validateConnectorPromotionFixture(reorderedGated).join("\n"),
    /canonical sorted connector ID order/,
  );
});

test("fails closed for missing, unknown, legacy, or mismatched native classifications", () => {
  const missing = {
    ...fixture,
    gatedNativeConfigConnectorIds: ["amazon_q", "cursor"],
  };
  assert.match(
    validateConnectorPromotionFixture(missing, frontend.expansionConnectorIds).join("\n"),
    /missing expansion connector IDs: qwen_code/,
  );

  const unknown = {
    ...fixture,
    gatedNativeConfigConnectorIds: [
      "amazon_q",
      "cursor",
      "qwen_code",
      "unknown_connector",
    ],
  };
  assert.match(
    validateConnectorPromotionFixture(unknown, frontend.expansionConnectorIds).join("\n"),
    /unknown expansion connector IDs: unknown_connector/,
  );

  const legacy = {
    ...fixture,
    gatedNativeConnectorIds: ["cursor"],
  };
  assert.match(
    validateConnectorPromotionFixture(legacy).join("\n"),
    /gatedNativeConnectorIds is unsupported/,
  );

  const mismatchedFrontend = {
    ...frontend,
    gatedNativeConfigConnectorIds: ["amazon_q", "cursor"],
  };
  assert.match(
    validateConnectorPromotionConsistency(fixture, mismatchedFrontend).join("\n"),
    /missing expansion connector IDs: qwen_code/,
  );
  assert.match(
    validateConnectorPromotionConsistency(fixture, mismatchedFrontend).join("\n"),
    /do not match the fixture/,
  );

  const unknownFrontend = {
    ...frontend,
    gatedNativeConfigConnectorIds: [
      "amazon_q",
      "cursor",
      "qwen_code",
      "unknown_connector",
    ],
  };
  assert.match(
    validateConnectorPromotionConsistency(fixture, unknownFrontend).join("\n"),
    /unknown expansion connector IDs: unknown_connector/,
  );

  const overlappingFrontend = {
    ...frontend,
    gatedNativeConfigConnectorIds: [
      "amazon_q",
      "cursor",
      "goose",
      "qwen_code",
    ],
  };
  assert.match(
    validateConnectorPromotionConsistency(fixture, overlappingFrontend).join("\n"),
    /promoted and gated connector IDs overlap: goose/,
  );

  const missingScope = { ...frontend, expansionConnectorIds: undefined };
  assert.match(
    validateConnectorPromotionConsistency(fixture, missingScope).join("\n"),
    /frontend expansionConnectorIds must be a non-empty array/,
  );
});
