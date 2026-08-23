// Content-free provider/tool registry for OSS harness interoperability.
// It describes capabilities only; it never stores secrets or enables writes.

const MAX_ENTRIES = 128;
const AUTH_SOURCES = new Set(["none", "keychain", "environment", "manual"]);
const FORBIDDEN_KEYS = new Set(["key", "token", "secret", "credential", "password", "authorization", "api_key", "apikey", "headers", "body"].map((key) => key.toLowerCase()));

function id(value, label) {
  if (typeof value !== "string" || value.trim() === "" || value.length > 128 || [...value].some((character) => character.charCodeAt(0) < 32)) throw new Error(`${label} must be a bounded identifier`);
  return value.trim();
}

function safeObject(value, path) {
  if (!value || typeof value !== "object") return;
  for (const [key, child] of Object.entries(value)) {
    if (FORBIDDEN_KEYS.has(key.toLowerCase())) throw new Error(`${path} contains forbidden secret field: ${key}`);
    if (child && typeof child === "object") safeObject(child, `${path}.${key}`);
  }
}

function uniqueIds(entries, label) {
  if (!Array.isArray(entries) || entries.length > MAX_ENTRIES) throw new Error(`${label} requires at most ${MAX_ENTRIES} entries`);
  const seen = new Set();
  for (const [index, entry] of entries.entries()) {
    safeObject(entry, `${label}[${index}]`);
    const entryId = id(entry.id, `${label}[${index}].id`);
    if (seen.has(entryId)) throw new Error(`duplicate ${label} id: ${entryId}`);
    seen.add(entryId);
  }
  return entries;
}

export function buildProviderToolRegistry({ providers = [], tools = [] }) {
  uniqueIds(providers, "provider");
  uniqueIds(tools, "tool");
  const providerIds = new Set(providers.map((provider) => provider.id.trim()));
  const normalizedProviders = providers.map((provider) => {
    const authSource = provider.authSource ?? "none";
    if (!AUTH_SOURCES.has(authSource)) throw new Error(`provider ${provider.id} has unsupported authSource`);
    if (!Array.isArray(provider.modelFamilies) || provider.modelFamilies.some((family) => typeof family !== "string" || family.trim() === "")) throw new Error(`provider ${provider.id} requires modelFamilies`);
    if (!Number.isSafeInteger(provider.contextLimit) || provider.contextLimit < 0) throw new Error(`provider ${provider.id} contextLimit must be a non-negative safe integer`);
    return { id: id(provider.id, "provider.id"), label: id(provider.label, "provider.label"), modelFamilies: [...new Set(provider.modelFamilies.map((family) => family.trim()))].sort(), contextLimit: provider.contextLimit, authSource };
  }).sort((left, right) => left.id.localeCompare(right.id));
  const normalizedTools = tools.map((tool) => {
    const providerId = id(tool.providerId, `tool ${tool.id}.providerId`);
    if (!providerIds.has(providerId)) throw new Error(`tool ${tool.id} references unknown provider`);
    if (!Array.isArray(tool.capabilities) || tool.capabilities.some((capability) => typeof capability !== "string" || capability.trim() === "")) throw new Error(`tool ${tool.id} requires capabilities`);
    return { id: id(tool.id, "tool.id"), label: id(tool.label, "tool.label"), providerId, capabilities: [...new Set(tool.capabilities.map((capability) => capability.trim()))].sort(), requiresApproval: tool.requiresApproval !== false, writesEnabled: false };
  }).sort((left, right) => left.id.localeCompare(right.id));
  return { schemaVersion: 1, registryMode: "metadata_only", writesEnabled: false, providers: normalizedProviders, tools: normalizedTools };
}

export function evaluateToolApproval(registry, toolId, approval) {
  const tool = registry.tools.find((candidate) => candidate.id === toolId);
  if (!tool) return { allowed: false, reason: "unknown_tool" };
  if (tool.writesEnabled || registry.writesEnabled) return { allowed: false, reason: "writes_disabled" };
  if (tool.requiresApproval && approval?.approved !== true) return { allowed: false, reason: "approval_required" };
  return { allowed: false, reason: "metadata_only_registry" };
}
