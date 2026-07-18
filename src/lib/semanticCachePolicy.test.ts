import { describe, expect, it } from "vitest";

import {
  buildSemanticCacheNamespaceKey,
  classifySemanticCacheRequest,
  createCacheReceipt,
} from "./semanticCachePolicy";

const base = {
  provider: "openai",
  model: "gpt-5",
  account: "acct-1",
  workspace: "/repo",
  prompt: "Summarize the changed files",
};

describe("semantic cache safety policy", () => {
  it("classifies exact and semantic requests", () => {
    expect(classifySemanticCacheRequest({ ...base, exactCacheEligible: true })).toEqual({ classification: "exact" });
    expect(classifySemanticCacheRequest(base)).toEqual({ classification: "semantic" });
  });

  it.each([
    ["streaming", { streaming: true }],
    ["tool/MCP", { hasToolCall: true }],
    ["MCP", { hasMcpCall: true }],
    ["sensitive", { sensitive: true }],
    ["no-cache", { noCache: true }],
    ["high temperature", { temperature: 0.21 }],
    ["rapid repo state", { repoStateChanging: true }],
    ["open tool calls", { openToolCalls: 1 }],
  ])("bypasses %s", (_label, overrides) => {
    expect(classifySemanticCacheRequest({ ...base, ...overrides })).toMatchObject({ classification: "bypass" });
  });

  it("namespaces provider, model, account, workspace, and policy", () => {
    expect(buildSemanticCacheNamespaceKey(base)).toBe("openai:gpt-5:acct-1:%2Frepo:semantic-cache-v1");
    expect(buildSemanticCacheNamespaceKey(base, { policyVersion: "v2", semanticTemperatureMax: 0.1 })).toContain(":v2");
  });

  it("labels cache outcomes and keeps evidence separate from compression", () => {
    expect(createCacheReceipt("cache-hit", "semantic", "observed")).toEqual({
      label: "cache-hit",
      classification: "semantic",
      evidence: "observed",
      compression: "separate",
    });
    expect(createCacheReceipt("bypass", "bypass")).toMatchObject({ evidence: "estimated", compression: "separate" });
  });
});
