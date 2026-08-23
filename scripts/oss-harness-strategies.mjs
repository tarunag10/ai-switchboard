// Deterministic, content-free Switchyard-style strategy fixtures.
// These return observations only; they never execute a provider route.

const MAX_CANDIDATES = 64;
const MAX_STAGES = 32;

function candidateList(candidates) {
  if (!Array.isArray(candidates) || candidates.length === 0 || candidates.length > MAX_CANDIDATES) {
    throw new Error(`strategy fixture requires 1-${MAX_CANDIDATES} candidates`);
  }
  const seen = new Set();
  return candidates.map((candidate, index) => {
    if (!candidate || typeof candidate !== "object" || typeof candidate.id !== "string" || candidate.id.trim() === "") {
      throw new Error(`candidate ${index} requires an id`);
    }
    const id = candidate.id.trim();
    if (seen.has(id)) throw new Error(`duplicate candidate id: ${id}`);
    seen.add(id);
    return { id, enabled: candidate.enabled !== false, healthy: candidate.healthy !== false };
  });
}

function observation(strategy, selectedCandidateId, reason, extra = {}) {
  return { strategy, executionMode: "observe_only", automaticPromotion: "disabled", selectedCandidateId, reason, ...extra };
}

export function deterministicRandomStrategy({ candidates, seed = 0 }) {
  const normalized = candidateList(candidates);
  if (!Number.isSafeInteger(seed) || seed < 0) throw new Error("random strategy seed must be a non-negative safe integer");
  const available = normalized.filter((candidate) => candidate.enabled && candidate.healthy);
  if (available.length === 0) return observation("random", null, "no_eligible_candidate");
  const state = (Math.imul(seed >>> 0, 1664525) + 1013904223) >>> 0;
  return observation("random", available[state % available.length].id, "deterministic_seed", { seed });
}

export function stagedStrategy({ stages, candidates }) {
  const normalized = candidateList(candidates);
  if (!Array.isArray(stages) || stages.length === 0 || stages.length > MAX_STAGES) throw new Error(`stage strategy requires 1-${MAX_STAGES} stages`);
  const byId = new Map(normalized.map((candidate) => [candidate.id, candidate]));
  for (const [index, stage] of stages.entries()) {
    if (!stage || typeof stage.id !== "string" || stage.id.trim() === "" || !Array.isArray(stage.candidateIds)) throw new Error(`stage ${index} requires an id and candidateIds`);
    const selected = stage.candidateIds.map((id) => byId.get(id)).find((candidate) => candidate?.enabled && candidate.healthy);
    if (selected) return observation("stage", selected.id, "first_eligible_stage", { stageId: stage.id.trim() });
  }
  return observation("stage", null, "no_eligible_stage");
}

export function escalationStrategy({ candidates, failureCount = 0 }) {
  const normalized = candidateList(candidates);
  if (!Number.isSafeInteger(failureCount) || failureCount < 0) throw new Error("failureCount must be a non-negative safe integer");
  const start = Math.min(failureCount, normalized.length - 1);
  const selected = normalized.slice(start).find((candidate) => candidate.enabled && candidate.healthy);
  return observation("escalation", selected?.id ?? null, selected ? "bounded_failure_escalation" : "escalation_exhausted", { failureCount });
}

export function fallbackStrategy({ primary, fallback, candidates }) {
  const normalized = candidateList(candidates);
  const byId = new Map(normalized.map((candidate) => [candidate.id, candidate]));
  const primaryCandidate = byId.get(primary);
  const fallbackCandidate = byId.get(fallback);
  if (!primaryCandidate || !fallbackCandidate) throw new Error("fallback strategy requires known primary and fallback candidates");
  if (primaryCandidate.enabled && primaryCandidate.healthy) return observation("fallback", primaryCandidate.id, "primary_healthy", { fallbackUsed: false });
  if (fallbackCandidate.enabled && fallbackCandidate.healthy) return observation("fallback", fallbackCandidate.id, "primary_unhealthy", { fallbackUsed: true });
  return observation("fallback", null, "primary_and_fallback_unhealthy", { fallbackUsed: false });
}
