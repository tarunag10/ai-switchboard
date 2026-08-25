#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import {
  extractConnectorPromotionFrontendContract,
  validateConnectorPromotionConsistency,
} from "./connector-promotion-contract.mjs";

const root = process.cwd();
const fixture = JSON.parse(
  fs.readFileSync(path.join(root, "fixtures/connector-promotion-evidence.json"), "utf8"),
);
const source = fs.readFileSync(path.join(root, "src/lib/plannedConnectors.ts"), "utf8");
let frontend;
try {
  frontend = extractConnectorPromotionFrontendContract(source);
} catch (error) {
  console.error(`connector promotion consistency failed: ${error.message}`);
  process.exit(1);
}

const errors = validateConnectorPromotionConsistency(fixture, frontend);
if (errors.length > 0) {
  console.error(`connector promotion consistency failed: ${errors.join("\n")}`);
  process.exit(1);
}
console.log(
  `Connector promotion consistency OK (${frontend.promotedNativeConnectorIds.length} promoted, ${frontend.gatedNativeConfigConnectorIds.length} gated, ${frontend.expansionConnectorIds.length} expansion connectors).`,
);
