export type ModeInspectorVerdict = "aligned" | "attention" | "blocked";

export interface ModeInspectorSignal {
  id: string;
  label: string;
  status: string;
  severity?: "info" | "warning" | "error";
}

export interface ModeInspectorVerdictInput {
  requestedMode: string;
  activeMode: string;
  proxyStatus: string;
  proxyAuthStatus?: string;
  staleShellWarning?: boolean;
  rows?: ModeInspectorSignal[];
}

export interface ModeInspectorVerdictResult {
  verdict: ModeInspectorVerdict;
  summary: string;
  attentionCount: number;
}

function normalizeMode(mode: string): string {
  return mode.trim().toLowerCase();
}

function rowNeedsAttention(row: ModeInspectorSignal): boolean {
  const status = row.status.toLowerCase();
  if (row.severity === "error") return true;
  if (row.severity === "warning") return true;
  return (
    status.includes("direct") ||
    status.includes("needs") ||
    status.includes("stale") ||
    status.includes("unknown") ||
    status.includes("failed") ||
    status.includes("blocked")
  );
}

export function deriveModeInspectorVerdict(
  input: ModeInspectorVerdictInput,
): ModeInspectorVerdictResult {
  const attentionRows = (input.rows ?? []).filter(rowNeedsAttention);
  const modeMismatch =
    normalizeMode(input.requestedMode) !== normalizeMode(input.activeMode);
  const proxyDown =
    input.proxyStatus.toLowerCase().includes("offline") ||
    input.proxyStatus.toLowerCase().includes("stopped");
  const proxyAuthRisk =
    input.proxyAuthStatus === "session_token_enforced" ||
    input.proxyAuthStatus === "loopback_validated_unauthenticated";

  if (modeMismatch || proxyDown) {
    return {
      verdict: "blocked",
      summary: modeMismatch
        ? "Requested and active modes differ."
        : "Headroom proxy is not reachable for the active mode.",
      attentionCount: attentionRows.length + 1,
    };
  }

  if (input.staleShellWarning || proxyAuthRisk || attentionRows.length > 0) {
    const reasons: string[] = [];
    if (input.staleShellWarning) reasons.push("stale shell exports");
    if (proxyAuthRisk) reasons.push("proxy auth needs attention");
    if (attentionRows.length > 0) reasons.push(`${attentionRows.length} routing row(s)`);
    return {
      verdict: "attention",
      summary: `Review ${reasons.join(", ")} before relying on savings.`,
      attentionCount: attentionRows.length + (input.staleShellWarning ? 1 : 0) + (proxyAuthRisk ? 1 : 0),
    };
  }

  return {
    verdict: "aligned",
    summary: "Requested mode, proxy, and routing evidence are aligned.",
    attentionCount: 0,
  };
}
