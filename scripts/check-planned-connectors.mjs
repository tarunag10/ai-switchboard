import fs from "node:fs";

const frontendPath = "src/lib/plannedConnectors.ts";
const backendPath = "src-tauri/src/client_adapters.rs";

function readFile(path) {
  return fs.readFileSync(path, "utf8");
}

function uniqueSorted(values) {
  return [...new Set(values)].sort();
}

function extractFrontendIds(source) {
  const plannedArray = source.match(
    /export const plannedConnectors: PlannedConnector\[] = \[([\s\S]*?)\];/,
  );
  if (!plannedArray) {
    throw new Error("Could not find plannedConnectors array in frontend source.");
  }

  return uniqueSorted(
    [...plannedArray[1].matchAll(/\bid:\s*"([^"]+)"/g)].map((match) => match[1]),
  );
}

function extractBackendIds(source) {
  const registry = source.match(
    /const PLANNED_CLIENT_SPECS:\s*\[PlannedClientSpec;\s*\d+\]\s*=\s*\[([\s\S]*?)\];/,
  );
  if (!registry) {
    throw new Error("Could not find PLANNED_CLIENT_SPECS registry in Rust source.");
  }

  return uniqueSorted(
    [...registry[1].matchAll(/\bid:\s*"([^"]+)"/g)].map((match) => match[1]),
  );
}

function difference(left, right) {
  const rightSet = new Set(right);
  return left.filter((item) => !rightSet.has(item));
}

const frontendIds = extractFrontendIds(readFile(frontendPath));
const backendIds = extractBackendIds(readFile(backendPath));

const frontendOnly = difference(frontendIds, backendIds);
const backendOnly = difference(backendIds, frontendIds);

if (frontendOnly.length > 0 || backendOnly.length > 0) {
  console.error("Planned connector registries are out of sync.");
  if (frontendOnly.length > 0) {
    console.error(`Only in ${frontendPath}: ${frontendOnly.join(", ")}`);
  }
  if (backendOnly.length > 0) {
    console.error(`Only in ${backendPath}: ${backendOnly.join(", ")}`);
  }
  process.exit(1);
}

if (frontendIds.length === 0) {
  console.error("No planned connectors found; expected at least one registry entry.");
  process.exit(1);
}

console.log(
  `Planned connector registries match (${frontendIds.length} connectors): ${frontendIds.join(", ")}`,
);
