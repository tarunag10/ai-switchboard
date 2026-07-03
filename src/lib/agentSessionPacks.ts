export interface AgentSessionPackCandidate {
  id: string;
  label: string;
  markdown: string;
  estimatedTokens: number;
  cacheableTokens?: number;
}

export interface AgentSessionPackRequest {
  agent: string;
  task: string;
  tokenBudget: number;
  preferredPackId?: string;
  candidates: AgentSessionPackCandidate[];
  enabled: boolean;
}

export interface AgentSessionPackPreparation {
  inject: boolean;
  packId: string | null;
  remainingBudget: number;
  stablePrefixMarkdown: string;
  reason: string;
  cacheableTokens: number;
}

export function prepareStartAgentSessionPack(
  request: AgentSessionPackRequest,
): AgentSessionPackPreparation {
  const tokenBudget = Math.max(0, request.tokenBudget);

  if (!request.enabled) {
    return emptyPreparation(tokenBudget, "pack_injection_disabled");
  }

  const selected =
    request.candidates.find((pack) => pack.id === request.preferredPackId) ??
    request.candidates.find((pack) => pack.id === "implementation") ??
    request.candidates[0];

  if (!selected) {
    return emptyPreparation(tokenBudget, "no_context_pack_available");
  }

  if (selected.estimatedTokens > tokenBudget) {
    return {
      inject: false,
      packId: selected.id,
      remainingBudget: tokenBudget,
      stablePrefixMarkdown: "",
      reason: "context_pack_exceeds_budget",
      cacheableTokens: 0,
    };
  }

  return {
    inject: true,
    packId: selected.id,
    remainingBudget: tokenBudget - selected.estimatedTokens,
    stablePrefixMarkdown: buildStablePrefix(request, selected),
    reason: "context_pack_injected",
    cacheableTokens: selected.cacheableTokens ?? selected.estimatedTokens,
  };
}

function emptyPreparation(
  tokenBudget: number,
  reason: string,
): AgentSessionPackPreparation {
  return {
    inject: false,
    packId: null,
    remainingBudget: tokenBudget,
    stablePrefixMarkdown: "",
    reason,
    cacheableTokens: 0,
  };
}

function buildStablePrefix(
  request: AgentSessionPackRequest,
  pack: AgentSessionPackCandidate,
): string {
  return [
    `# Start Agent Session: ${request.agent}`,
    "",
    `Task: ${request.task}`,
    `Injected pack: ${pack.label}`,
    `Estimated tokens: ${pack.estimatedTokens}`,
    "",
    "<stable-context-pack>",
    pack.markdown.trim(),
    "</stable-context-pack>",
  ].join("\n");
}
