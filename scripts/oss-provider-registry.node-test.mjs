import test from "node:test";
import assert from "node:assert/strict";
import { buildProviderToolRegistry, evaluateToolApproval } from "./oss-provider-registry.mjs";

const input = {
  providers: [{ id: "anthropic", label: "Anthropic", modelFamilies: ["frontier", "frontier"], contextLimit: 200000, authSource: "keychain" }],
  tools: [{ id: "repo-context", label: "Repo context", providerId: "anthropic", capabilities: ["context", "context"], requiresApproval: true }],
};

test("builds deterministic metadata-only provider and tool registry", () => {
  const registry = buildProviderToolRegistry(input);
  assert.equal(registry.registryMode, "metadata_only");
  assert.equal(registry.writesEnabled, false);
  assert.deepEqual(registry.providers[0].modelFamilies, ["frontier"]);
  assert.equal(registry.tools[0].writesEnabled, false);
});

test("approval evaluation fails closed even with approval", () => {
  const registry = buildProviderToolRegistry(input);
  assert.deepEqual(evaluateToolApproval(registry, "repo-context", { approved: true }), { allowed: false, reason: "metadata_only_registry" });
  assert.deepEqual(evaluateToolApproval(registry, "missing", { approved: true }), { allowed: false, reason: "unknown_tool" });
});

test("rejects secrets, duplicate IDs, unknown providers, and unsupported auth", () => {
  assert.throws(() => buildProviderToolRegistry({ providers: [{ id: "x", label: "X", modelFamilies: ["m"], contextLimit: 1, apiKey: "secret" }] }), /forbidden secret/);
  assert.throws(() => buildProviderToolRegistry({ providers: [input.providers[0], input.providers[0]] }), /duplicate provider/);
  assert.throws(() => buildProviderToolRegistry({ providers: [{ ...input.providers[0], authSource: "token" }] }), /unsupported authSource/);
  assert.throws(() => buildProviderToolRegistry({ providers: input.providers, tools: [{ ...input.tools[0], providerId: "missing" }] }), /unknown provider/);
});
