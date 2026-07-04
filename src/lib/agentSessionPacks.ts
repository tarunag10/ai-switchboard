export interface AgentSessionPackCandidate {
  id: string;
  name: string;
  summary: string;
  estimatedTokens: number;
  stablePrefix: string;
  cacheableTokens?: number;
}

export interface AgentSessionPackRequest {
  agentId: string;
  task: string;
  tokenBudget: number;
  enabled: boolean;
  preferredPackId?: string;
  candidates: AgentSessionPackCandidate[];
}

export type AgentSessionPackReason =
  | "context_pack_injected"
  | "context_pack_exceeds_budget"
  | "no_context_pack_available"
  | "pack_injection_disabled";

export interface AgentSessionPackPreparation {
  inject: boolean;
  packId?: string;
  packName?: string;
  reason: AgentSessionPackReason;
  remainingBudget: number;
  stablePrefixMarkdown: string;
  cacheableTokens: number;
}

export interface AgentSessionPreset {
  id: string;
  label: string;
  defaultBudget: number;
  packs: AgentSessionPackCandidate[];
}

export const AGENT_SESSION_PRESETS: AgentSessionPreset[] = [
  {
    id: "codex",
    label: "Codex",
    defaultBudget: 24_000,
    packs: [
      {
        id: "implementation",
        name: "Implementation Pack",
        summary: "Repo conventions, current plan, target files, and proof steps.",
        estimatedTokens: 3_200,
        cacheableTokens: 2_850,
        stablePrefix:
          "Repo: mac-ai-switchboard\nMode: implementation\nKeep changes scoped, use rtk-prefixed commands, avoid reverting others, and report tests plus blockers.",
      },
      {
        id: "handoff",
        name: "Handoff Pack",
        summary: "Current status, decisions, remaining risks, and next commands.",
        estimatedTokens: 1_550,
        cacheableTokens: 1_350,
        stablePrefix:
          "Repo: mac-ai-switchboard\nMode: handoff\nPreserve exact cwd, touched files, verification commands, and unresolved blockers.",
      },
    ],
  },
  {
    id: "claude",
    label: "Claude Code",
    defaultBudget: 18_000,
    packs: [
      {
        id: "review",
        name: "Review Pack",
        summary: "Diff boundaries, regression risks, and local verification context.",
        estimatedTokens: 2_400,
        cacheableTokens: 2_050,
        stablePrefix:
          "Repo: mac-ai-switchboard\nMode: review\nPrioritize correctness, regressions, missing tests, and actionable file-line findings.",
      },
      {
        id: "docs",
        name: "Docs Pack",
        summary: "Product language, docs truth, release ledger, and command evidence.",
        estimatedTokens: 1_850,
        cacheableTokens: 1_600,
        stablePrefix:
          "Repo: mac-ai-switchboard\nMode: docs\nKeep docs aligned with implemented behavior and cite concrete artifact paths.",
      },
    ],
  },
];

export function prepareStartAgentSessionPack(
  request: AgentSessionPackRequest,
): AgentSessionPackPreparation {
  const tokenBudget = Math.max(0, Math.floor(request.tokenBudget));

  if (!request.enabled) {
    return emptyPreparation(tokenBudget, "pack_injection_disabled");
  }

  const selected =
    request.candidates.find((pack) => pack.id === request.preferredPackId) ??
    request.candidates[0];

  if (!selected) {
    return emptyPreparation(tokenBudget, "no_context_pack_available");
  }

  if (selected.estimatedTokens > tokenBudget) {
    return {
      inject: false,
      packId: selected.id,
      packName: selected.name,
      reason: "context_pack_exceeds_budget",
      remainingBudget: tokenBudget,
      stablePrefixMarkdown: "",
      cacheableTokens: 0,
    };
  }

  return {
    inject: true,
    packId: selected.id,
    packName: selected.name,
    reason: "context_pack_injected",
    remainingBudget: tokenBudget - selected.estimatedTokens,
    stablePrefixMarkdown: formatStablePrefixMarkdown(request, selected),
    cacheableTokens: selected.cacheableTokens ?? selected.estimatedTokens,
  };
}

export function buildAgentSessionPayload(
  request: AgentSessionPackRequest,
): string {
  const preparation = prepareStartAgentSessionPack(request);
  const selected =
    request.candidates.find((pack) => pack.id === preparation.packId) ??
    request.candidates.find((pack) => pack.id === request.preferredPackId);

  const payload = {
    action: "start_agent_session",
    agent: request.agentId,
    task: request.task.trim(),
    tokenBudget: Math.max(0, Math.floor(request.tokenBudget)),
    pack: selected
      ? {
          id: selected.id,
          name: selected.name,
          estimatedTokens: selected.estimatedTokens,
          cacheableTokens: selected.cacheableTokens ?? selected.estimatedTokens,
        }
      : null,
    injectStablePrefix: preparation.inject,
    remainingBudget: preparation.remainingBudget,
    reason: preparation.reason,
    stablePrefixMarkdown: preparation.stablePrefixMarkdown,
  };

  return JSON.stringify(payload, null, 2);
}

export function getAgentSessionActionLabel(
  preparation: AgentSessionPackPreparation,
): string {
  switch (preparation.reason) {
    case "context_pack_injected":
      return "Ready to copy payload";
    case "context_pack_exceeds_budget":
      return "Increase budget or pick a smaller pack";
    case "no_context_pack_available":
      return "No pack available";
    case "pack_injection_disabled":
      return "Pack injection disabled";
  }
}

function emptyPreparation(
  tokenBudget: number,
  reason: AgentSessionPackReason,
): AgentSessionPackPreparation {
  return {
    inject: false,
    reason,
    remainingBudget: tokenBudget,
    stablePrefixMarkdown: "",
    cacheableTokens: 0,
  };
}

function formatStablePrefixMarkdown(
  request: AgentSessionPackRequest,
  pack: AgentSessionPackCandidate,
): string {
  return [
    "<stable-context-pack>",
    `agent: ${request.agentId}`,
    `pack: ${pack.id}`,
    `task: ${request.task.trim() || "unspecified"}`,
    "",
    pack.stablePrefix.trim(),
    "</stable-context-pack>",
  ].join("\n");
}
