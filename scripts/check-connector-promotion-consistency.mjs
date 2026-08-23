#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const fixture = JSON.parse(
  fs.readFileSync(path.join(root, "fixtures/connector-promotion-evidence.json"), "utf8"),
);
const source = fs.readFileSync(path.join(root, "src/lib/plannedConnectors.ts"), "utf8");
const match = source.match(
  /export const promotedNativeConfigConnectorIds = new Set\(\[([\s\S]*?)\]\);/,
);
if (!match) {
  console.error("connector promotion consistency failed: planned native promotion set is missing");
  process.exit(1);
}
const planned = [...match[1].matchAll(/"([a-z0-9_]+)"/g)].map((item) => item[1]).sort();
const canonical = [...(fixture.promotedNativeConnectorIds ?? [])].sort();
if (JSON.stringify(planned) !== JSON.stringify(canonical)) {
  console.error(
    `connector promotion consistency failed: UI=[${planned.join(", ")}] fixture=[${canonical.join(", ")}]`,
  );
  process.exit(1);
}
console.log(`Connector promotion consistency OK (${canonical.join(", ")}).`);
