import crypto from "node:crypto";

const FORBIDDEN_KEYS = new Set([
  "prompt", "messages", "input", "output", "response", "body", "headers",
  "authorization", "apiKey", "api_key", "token", "secret", "credential",
]);
const ROUTES = new Set(["headroom", "direct_anthropic", "direct_openai", "cache", "switchyard_observe"]);
const OUTCOMES = new Set(["success", "upstream_http_error", "connect_failure", "write_failure", "read_failure", "timeout", "client_disconnect", "local_rejection"]);

function rejectSensitive(value, path = "event") {
  if (!value || typeof value !== "object") return;
  for (const [key, child] of Object.entries(value)) {
    if (FORBIDDEN_KEYS.has(key)) throw new Error(`sensitive field is not allowed: ${path}.${key}`);
    rejectSensitive(child, `${path}.${key}`);
  }
}

function validateEvent(event, index) {
  if (!event || typeof event !== "object" || Array.isArray(event)) throw new Error(`event ${index} must be an object`);
  rejectSensitive(event, `events[${index}]`);
  for (const key of ["eventId", "taskClass", "route", "outcome"]) {
    if (typeof event[key] !== "string" || event[key].trim() === "") throw new Error(`event ${index} requires ${key}`);
  }
  if (!ROUTES.has(event.route)) throw new Error(`event ${index} has unsupported route`);
  if (!OUTCOMES.has(event.outcome)) throw new Error(`event ${index} has unsupported outcome`);
  if (event.latencyMs !== undefined && (!Number.isSafeInteger(event.latencyMs) || event.latencyMs < 0)) {
    throw new Error(`event ${index} latencyMs must be a non-negative safe integer`);
  }
}

export function replayRedactedRouteEvents(input) {
  if (!input || input.schemaVersion !== 1 || !Array.isArray(input.events)) {
    throw new Error("replay input requires schemaVersion 1 and an events array");
  }
  input.events.forEach(validateEvent);
  const canonical = input.events.map(({ eventId, taskClass, route, outcome, latencyMs = null }) => ({ eventId, taskClass, route, outcome, latencyMs }));
  const routeCounts = Object.fromEntries([...ROUTES].map((route) => [route, 0]));
  const outcomeCounts = Object.fromEntries([...OUTCOMES].map((outcome) => [outcome, 0]));
  const latencies = [];
  for (const event of canonical) {
    routeCounts[event.route] += 1;
    outcomeCounts[event.outcome] += 1;
    if (event.latencyMs !== null) latencies.push(event.latencyMs);
  }
  latencies.sort((a, b) => a - b);
  const p95Index = latencies.length ? Math.min(latencies.length - 1, Math.ceil(latencies.length * 0.95) - 1) : null;
  const digest = crypto.createHash("sha256").update(JSON.stringify(canonical)).digest("hex");
  return {
    schemaVersion: 1,
    replayMode: "redacted_observe_only",
    automaticPromotion: "disabled",
    providerTraffic: "none",
    eventCount: canonical.length,
    routeCounts,
    outcomeCounts,
    latency: { sampleCount: latencies.length, p95Ms: p95Index === null ? null : latencies[p95Index] },
    replayDigest: `sha256:${digest}`,
  };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const raw = await import("node:fs/promises").then((fs) => fs.readFile(process.argv[2], "utf8"));
  console.log(JSON.stringify(replayRedactedRouteEvents(JSON.parse(raw)), null, 2));
}

