import {
  connectorControlState,
  formatConnectorConfigDryRunPreview,
} from "./dashboardHelpers";
import {
  supportsDedicatedCleanupRollbackRecord,
  supportsNativeManagedRollbackRecord,
} from "./managedChanges";
import { formatPlannedConnectorConfigCreationPlansMarkdown } from "./plannedConnectors";
import type { ClientConnectorStatus } from "./types";
import type { ManagedChangeRecord } from "./managedChanges";
import type { ConnectorDossier } from "./plannedConnectors";

export const connectorSetupDetails: Record<string, string> = {
  claude_code:
    "Headroom injects ANTHROPIC_BASE_URL into shell profiles and ~/.claude/settings.json so Claude Code connects through Headroom. Token-saving add-ons like RTK are optional.",
  codex:
    "Headroom writes a managed provider block to ~/.codex/config.toml and exports OPENAI_BASE_URL in shell profiles so Codex connects through Headroom.",
  gemini_cli:
    "Switchboard can configure Gemini CLI with managed shell routing, backup, Doctor verification, rollback, and Off cleanup.",
  opencode:
    "Switchboard can configure OpenCode with a managed provider entry, backup, Doctor verification, rollback, and Off cleanup.",
  cursor:
    "Cursor is detected and shown with a manual guide. Switchboard does not change Cursor provider settings yet because profile and account behavior can vary by release channel.",
  grok_cli:
    "Grok / xAI CLI uses the documented ~/.grok/config.toml [endpoints].models_base_url field for reversible native routing; XAI_API_KEY/login, account state, and model selection remain manual.",
  aider:
    "Aider is detected when installed. RTK-only mode can already reduce noisy shell output while provider setup remains manual.",
  continue:
    "Continue is detected when installed. Provider setup stays manual until Switchboard can preserve and restore Continue config safely.",
  goose:
    "Goose can use the managed Repo Memory MCP bridge for read-only context handoff; Switchboard manages only allowlisted OpenAI/Anthropic endpoint fields while credentials, account state, and model selection stay manual.",
  qwen_code:
    "Qwen Code has a Switchboard-owned sidecar path for handoff/routing evidence. Account and model setup stay manual.",
  amazon_q:
    "Amazon Q Developer CLI is detected when installed. Verification packs are safe today; AWS credentials, SSO, and profiles stay manual.",
  windsurf:
    "Windsurf is a managed editor connector. Switchboard manages editor settings routing with backups, verification, rollback, and Off cleanup.",
  zed_ai:
    "Zed AI is a managed editor connector. Switchboard manages assistant settings routing with backups, verification, rollback, and Off cleanup.",
};

export const connectorUnavailableReasons: Record<string, string> = {
  claude_code:
    "Claude Code was not detected. Install Claude Code, then reopen AI Switchboard for Mac.",
  codex:
    "Codex was not detected. Install the Codex CLI, then reopen AI Switchboard for Mac.",
  gemini_cli:
    "Gemini CLI was not detected. Install Gemini CLI, then reopen AI Switchboard for Mac.",
  opencode:
    "OpenCode was not detected. Install OpenCode, then reopen AI Switchboard for Mac.",
  cursor:
    "Cursor automatic setup is off for now. Open Cursor settings and keep provider/model choices manual.",
  grok_cli:
    "Grok / xAI CLI endpoint routing is managed when ~/.grok/config.toml is available; keep XAI_API_KEY/login, model, and account choices manual.",
  aider:
    "Aider automatic setup is off for now. Use RTK-only mode or copied Repo Intelligence packs.",
  continue:
    "Continue automatic setup is off for now. Review provider config manually.",
  goose:
    "Goose credentials, account state, and model selection stay manual. Switchboard manages only documented endpoint fields and the Repo Memory MCP bridge.",
  qwen_code:
    "Qwen Code account and model setup are manual. Switchboard only manages its own sidecar evidence.",
  amazon_q:
    "Amazon Q automatic setup is off for now. Keep AWS credentials, SSO, and profiles manual.",
  windsurf:
    "Windsurf was not detected. Install Windsurf, then reopen AI Switchboard for Mac.",
  zed_ai: "Zed was not detected. Install Zed, then reopen AI Switchboard for Mac.",
};

export function firstManagedConfigTarget(record: ManagedChangeRecord) {
  return record.paths[0] ?? "~/.config/mac-ai-switchboard-managed";
}

export function supportsNativeManagedRollback(record: ManagedChangeRecord) {
  return (
    supportsNativeManagedRollbackRecord(record.id) ||
    supportsDedicatedCleanupRollbackRecord(record.id)
  );
}

export function supportsNativeConfigApply(record: ManagedChangeRecord) {
  return (
    record.id === "opencode-routing" ||
    record.id === "grok-routing" ||
    record.id === "goose-provider-routing"
  );
}

export function getConnectorUnavailableReason(
  connector: ClientConnectorStatus,
) {
  return connectorControlState(connector).reason;
}

export function getConnectorDetectionWarning(connector: ClientConnectorStatus) {
  if (connector.installed) {
    return null;
  }
  return connectorUnavailableReasons[connector.clientId] ?? null;
}

export function getPlannedConnectorNextStep(
  connector: ClientConnectorStatus,
  plannedConnector: ConnectorDossier,
) {
  if (!connector.installed) {
    return "Install the tool first, then Switchboard will detect it here.";
  }

  if (plannedConnector.setupPhase === "Managed") {
    return "Detected. Managed routing can be repaired by Doctor if setup drifts.";
  }

  if (plannedConnector.setupPhase === "Detect") {
    return "Detected. Keep using RTK-only mode while a reversible routing adapter is researched.";
  }

  if (plannedConnector.setupPhase === "Guide") {
    return "Detected. App-guided setup is next so account-specific provider settings stay under your control.";
  }

  return "Detected. Automatic setup waits for backup, restore, and off-mode cleanup coverage.";
}

export function formatBackendConnectorConfigPlan(
  connector: ClientConnectorStatus,
  plannedConnector: ConnectorDossier,
) {
  const stepDetails = connector.configCreationStepDetails ?? [];
  const stepLabels = connector.configCreationSteps ?? [];
  if (stepDetails.length === 0 && stepLabels.length === 0) {
    return formatPlannedConnectorConfigCreationPlansMarkdown([
      plannedConnector,
    ]);
  }

  return [
    "# AI Switchboard Connector Config Creation Plan",
    "",
    `## ${connector.name}`,
    "- Automation enabled: no",
    "- Safety note: Automatic setup stays off until every step has tests and Doctor evidence.",
    ...(stepDetails.length > 0
      ? stepDetails.map((step) => {
          const evidence = step.requiredEvidence?.length
            ? ` Required evidence: ${step.requiredEvidence.join(" ")}`
            : "";
          return `- ${step.label}: ${step.detail}${evidence}`;
        })
      : stepLabels.map((step) => `- ${step}`)),
    "",
    formatConnectorConfigDryRunPreview(connector),
  ].join("\n");
}
