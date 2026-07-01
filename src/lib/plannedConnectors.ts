import connectorManifestJson from "../../connectors/manifest.json";

export type ConnectorSupportStatus =
  | "managed"
  | "guided"
  | "detected"
  | "planned"
  | "unsupported";

export interface ConnectorManifest {
  id: string;
  name: string;
  category: "cli" | "editor" | "agent" | "runtime";
  support_status: ConnectorSupportStatus;
  detection: {
    binaries?: string[];
    paths?: string[];
  };
  config?: {
    locations?: string[];
    forbidden_reads?: string[];
  };
  automation_gates: string[];
  manual_workflow: string[];
}

export interface ConnectorSupportMatrixRow {
  id: string;
  name: string;
  category: ConnectorManifest["category"];
  supportStatus: ConnectorSupportStatus;
  detectionSources: string[];
  configLocations: string[];
  automationGateCount: number;
  manualWorkflow: string[];
}

export const connectorManifests =
  connectorManifestJson as ConnectorManifest[];

const connectorManifestById = new Map(
  connectorManifests.map((manifest) => [manifest.id, manifest]),
);

export function getConnectorManifest(id: string): ConnectorManifest | null {
  return connectorManifestById.get(id) ?? null;
}

export function connectorSupportMatrixRows(): ConnectorSupportMatrixRow[] {
  return connectorManifests.map((manifest) => ({
    id: manifest.id,
    name: manifest.name,
    category: manifest.category,
    supportStatus: manifest.support_status,
    detectionSources: [
      ...(manifest.detection.binaries ?? []).map((binary) => `PATH: ${binary}`),
      ...(manifest.detection.paths ?? []),
    ],
    configLocations: manifest.config?.locations ?? [],
    automationGateCount: manifest.automation_gates.length,
    manualWorkflow: manifest.manual_workflow,
  }));
}

export interface PlannedConnector {
  id: string;
  name: string;
  category: "cli" | "editor" | "agent";
  supportStatus: ConnectorSupportStatus;
  statusLabel: "Gated";
  setupPhase: "Detect" | "Guide" | "Adapt";
  integrationTarget: string;
  notes: string;
  capabilityBadges: string[];
  capabilityRows: PlannedConnectorCapability[];
  supportedModes: string[];
  safeToday: string;
  firstAutomation: string;
  configSurfaces: string[];
  automationGates: string[];
  manualWorkflow: string[];
}

export type ManagedConnectorDossier = Omit<
  PlannedConnector,
  "statusLabel" | "setupPhase"
> & {
  supportStatus: "managed";
  statusLabel: "Managed";
  setupPhase: "Managed";
};

export type ConnectorDossier = PlannedConnector | ManagedConnectorDossier;

export interface PlannedConnectorCapability {
  label: string;
  state: "Available now" | "Manual today" | "Gated";
  detail: string;
}

export interface PlannedConnectorSupportSummary {
  connectorCount: number;
  safeTodayCount: number;
  manualTodayCount: number;
  plannedCount: number;
  automationGateCount: number;
  safeTodayLabels: string[];
  plannedLabels: string[];
}

export interface PlannedConnectorSetupGuide {
  label: string;
  command: string;
  notes: string;
}

export type PlannedConnectorReadinessStageId =
  | "detected"
  | "manualGuide"
  | "backupImplemented"
  | "applyImplemented"
  | "verifyImplemented"
  | "rollbackImplemented"
  | "offCleanupImplemented";

export interface PlannedConnectorReadinessStage {
  id: PlannedConnectorReadinessStageId;
  label: string;
  state: "ready" | "blocked";
  evidence: string;
}

export interface PlannedConnectorReadinessContract {
  connectorId: string;
  connectorName: string;
  setupPhase: PlannedConnector["setupPhase"] | "Managed";
  automationEnabled: boolean;
  nextBlockedStage: PlannedConnectorReadinessStageId | null;
  stages: PlannedConnectorReadinessStage[];
}

export type PlannedConnectorReadinessBadgeKind =
  | "manual-only"
  | "automation-gated"
  | "verified-automation"
  | "unsupported-account-model";

export interface PlannedConnectorReadinessBadge {
  kind: PlannedConnectorReadinessBadgeKind;
  label: string;
  detail: string;
}

export interface PlannedConnectorSafetyDossier {
  connectorId: string;
  configPathStrategy: string;
  providerSemantics: string;
  accountCaveat: string;
  rollbackStrategy: string;
}

export interface PlannedConnectorConfigCreationStep {
  id:
    | "detect"
    | "dryRunDiff"
    | "backup"
    | "apply"
    | "verify"
    | "rollback"
    | "offCleanup";
  label: string;
  detail: string;
  requiredEvidence: string[];
}

export interface PlannedConnectorConfigCreationPlan {
  connectorId: string;
  connectorName: string;
  automationEnabled: boolean;
  safetyNote: string;
  steps: PlannedConnectorConfigCreationStep[];
}

export const plannedConnectorReadinessStageOrder: PlannedConnectorReadinessStageId[] =
  [
    "detected",
    "manualGuide",
    "backupImplemented",
    "applyImplemented",
    "verifyImplemented",
    "rollbackImplemented",
    "offCleanupImplemented",
  ];

export const managedConnectorDossiers: ManagedConnectorDossier[] = [
  {
    id: "gemini_cli",
    name: "Gemini CLI",
    category: "cli",
    supportStatus: "managed",
    statusLabel: "Managed",
    setupPhase: "Managed",
    integrationTarget: "Managed shell base-url routing adapter.",
    notes:
      "Switchboard manages Gemini CLI routing through shell exports, Doctor verification, rollback, restore, and Off cleanup.",
    capabilityBadges: [
      "Managed routing",
      "Doctor verified",
      "Rollback ready",
    ],
    supportedModes: ["Full", "Headroom", "Off"],
    safeToday:
      "Enable the connector to write managed Gemini CLI routing exports and rollback evidence.",
    firstAutomation:
      "Doctor re-applies the managed shell routing block if verification drifts.",
    capabilityRows: [
      {
        label: "Managed routing",
        state: "Available now",
        detail:
          "Switchboard writes managed Gemini CLI base-url and proxy API-key shell exports.",
      },
      {
        label: "Verification",
        state: "Available now",
        detail:
          "Doctor verifies the managed shell exports and sibling rollback backup.",
      },
      {
        label: "Rollback",
        state: "Available now",
        detail:
          "Off mode removes only Switchboard-owned Gemini shell routing exports.",
      },
    ],
    configSurfaces: [
      "Gemini CLI binary",
      "provider settings",
      "shell environment",
    ],
    automationGates: [
      "Write only Switchboard-owned shell blocks and sibling rollback backups.",
      "Verify GOOGLE_GEMINI_BASE_URL, GEMINI_BASE_URL, and GEMINI_API_KEY routing exports.",
      "Off mode removes local proxy routing without changing account state.",
    ],
    manualWorkflow: [
      "Confirm the Gemini CLI binary is installed.",
      "Toggle the connector on from Settings.",
      "Use Doctor repair if managed Gemini routing drifts.",
    ],
  },
  {
    id: "opencode",
    name: "OpenCode",
    category: "cli",
    supportStatus: "managed",
    statusLabel: "Managed",
    setupPhase: "Managed",
    integrationTarget: "Managed OpenCode provider config adapter.",
    notes:
      "Switchboard manages an OpenCode headroom provider with backups, Doctor verification, rollback, restore, and Off cleanup.",
    capabilityBadges: [
      "Managed provider",
      "Doctor verified",
      "Rollback ready",
    ],
    supportedModes: ["Full", "Headroom", "Off"],
    safeToday:
      "Enable the connector to write the managed OpenCode provider and rollback evidence.",
    firstAutomation:
      "Doctor re-applies the managed provider block if verification drifts.",
    capabilityRows: [
      {
        label: "Managed provider",
        state: "Available now",
        detail:
          "Switchboard writes a Headroom provider in ~/.config/opencode/opencode.json.",
      },
      {
        label: "Verification",
        state: "Available now",
        detail:
          "Doctor verifies the OpenCode provider baseURL and sibling rollback backup.",
      },
      {
        label: "Rollback",
        state: "Available now",
        detail:
          "Off mode removes only the Switchboard-owned OpenCode provider from native config.",
      },
    ],
    configSurfaces: ["OpenCode binary", "provider config", "shell environment"],
    automationGates: [
      "Create timestamped backups before provider edits.",
      "Verify the managed headroom provider points at the local proxy.",
      "Prove Off mode removes only the Switchboard-owned provider config.",
    ],
    manualWorkflow: [
      "Confirm OpenCode is installed.",
      "Toggle the connector on from Settings.",
      "Use Doctor repair if managed config drifts.",
    ],
  },
  {
    id: "windsurf",
    name: "Windsurf",
    category: "editor",
    supportStatus: "managed",
    statusLabel: "Managed",
    setupPhase: "Managed",
    integrationTarget: "Managed Windsurf editor settings routing adapter.",
    notes: "Switchboard manages Windsurf editor settings routing with backups, Doctor verification, rollback, and Off cleanup.",
    capabilityBadges: [
      "Managed routing",
      "Doctor verified",
      "Rollback ready",
    ],
    supportedModes: ["Full", "Headroom", "Off"],
    safeToday: "Enable the connector to write managed Windsurf editor settings routing and rollback evidence.",
    firstAutomation: "Doctor re-applies the managed Windsurf routing block if verification drifts.",
    capabilityRows: [
      {
        label: "Managed routing",
        state: "Available now",
        detail: "Switchboard writes managed Windsurf editor settings routing.",
      },
      {
        label: "Verification",
        state: "Available now",
        detail: "Doctor verifies the managed Windsurf settings block and sibling rollback backup.",
      },
      {
        label: "Rollback",
        state: "Available now",
        detail: "Off mode removes only Switchboard-owned Windsurf config blocks.",
      },
    ],
    configSurfaces: ["Windsurf app bundle", "user settings", "profile settings"],
    automationGates: [
      "Back up Windsurf settings before edits.",
      "Verify managed Windsurf routing block.",
      "Rollback restores settings from backup.",
      "Off mode removes only Switchboard-owned managed blocks.",
    ],
    manualWorkflow: [
      "Confirm Windsurf is installed.",
      "Toggle the connector on from Settings.",
      "Use Doctor repair if managed config drifts.",
    ],
  },
  {
    id: "zed_ai",
    name: "Zed AI",
    category: "editor",
    supportStatus: "managed",
    statusLabel: "Managed",
    setupPhase: "Managed",
    integrationTarget: "Managed Zed assistant settings routing adapter.",
    notes: "Writes managed assistant settings routing to ~/.config/zed/settings.json with full backup/verify/rollback.",
    capabilityBadges: [
      "Managed routing",
      "Doctor verified",
      "Rollback ready",
    ],
    supportedModes: ["Full", "Headroom", "Off"],
    safeToday: "Enable the connector to write the managed Zed routing block and rollback evidence.",
    firstAutomation: "Doctor re-applies the managed routing block if verification drifts.",
    capabilityRows: [
      {
        label: "Managed routing",
        state: "Available now",
        detail: "Switchboard writes a managed routing block to ~/.config/zed/settings.json.",
      },
      {
        label: "Verification",
        state: "Available now",
        detail: "Doctor verifies the managed Zed settings block and sibling rollback backup.",
      },
      {
        label: "Rollback",
        state: "Available now",
        detail: "Off mode removes only the Switchboard-owned routing block.",
      },
    ],
    configSurfaces: [
      "Zed app bundle",
      "~/.config/zed/settings.json",
      "~/Library/Application Support/Zed",
    ],
    automationGates: [
      "Detect Zed settings.json before injecting routing block.",
      "Preserve unknown settings losslessly.",
      "Restore settings from backup.",
      "Verify managed assistant settings routing after apply.",
      "Clean up managed routing block on disconnect.",
    ],
    manualWorkflow: [
      "Toggle the connector on from Settings.",
      "Restart Zed to pick up the injected routing block.",
      "Verify a prompt routes through Headroom.",
    ],
  },
];

export const plannedConnectors: PlannedConnector[] = [
  {
    id: "cursor",
    name: "Cursor",
    category: "editor",
    supportStatus: "planned",
    statusLabel: "Gated",
    setupPhase: "Guide",
    integrationTarget:
      "Editor settings/profile detection with opt-in local proxy routing where supported.",
    notes:
      "Treat as guided setup first because Cursor settings and extension behavior can vary by account release channel.",
    capabilityBadges: [
      "App detection",
      "Guided setup",
      "Settings discovery",
    ],
    supportedModes: ["Repo packs", "Guided setup", "Off"],
    safeToday:
      "Show Cursor as a guided editor target and let users copy Repo Intelligence packs into sessions.",
    firstAutomation:
      "Add profile-aware settings discovery with a dry-run diff before any settings write.",
    capabilityRows: [
      {
        label: "App detection",
        state: "Available now",
        detail: "Switchboard can show Cursor as a gated editor connector.",
      },
      {
        label: "Manual setup",
        state: "Manual today",
        detail:
          "Open Cursor settings and review model/provider routing manually.",
      },
      {
        label: "Settings adapter",
        state: "Available now",
        detail:
          "Settings file discovery is available; writes still wait for parse, backup, restore, and Off cleanup tests.",
      },
    ],
    configSurfaces: ["Cursor app bundle", "user settings", "profile settings"],
    automationGates: [
      "Detect the active Cursor profile before reading settings.",
      "Back up settings without touching extension-managed secrets.",
      "Keep account-specific model choices visible before routing.",
    ],
    manualWorkflow: [
      "Open Cursor settings from the setup guide.",
      "Review provider/model settings manually.",
      "Use Repo Intelligence packs as copyable context until editor handoff is stable.",
    ],
  },
  {
    id: "grok_cli",
    name: "Grok / xAI CLI",
    category: "cli",
    supportStatus: "planned",
    statusLabel: "Gated",
    setupPhase: "Detect",
    integrationTarget:
      "Provider/base-url adapter once a stable local CLI surface is identified.",
    notes:
      "Track separately from generic OpenAI-compatible clients so account/model constraints stay visible in Doctor.",
    capabilityBadges: [
      "CLI detection",
      "Model guardrails gated",
      "Provider routing gated",
    ],
    supportedModes: ["RTK only", "Off"],
    safeToday:
      "Detect grok or xai commands and keep model/provider choices visible instead of auto-routing.",
    firstAutomation:
      "Add Doctor model/account guardrails before a local provider adapter is offered.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can check for local grok or xai commands.",
      },
      {
        label: "Model guardrails",
        state: "Gated",
        detail:
          "Doctor should prevent unsupported model/account combinations before routing.",
      },
      {
        label: "Provider routing",
        state: "Gated",
        detail:
          "Automatic setup waits for a stable OpenAI-compatible local config surface.",
      },
    ],
    configSurfaces: [
      "grok or xai binary",
      "provider/model flags",
      "shell environment",
    ],
    automationGates: [
      "Detect a stable xAI CLI surface.",
      "Add Doctor guardrails for unsupported model/account combinations.",
      "Keep API key and account state outside managed app storage.",
    ],
    manualWorkflow: [
      "Confirm whether grok or xai exists locally.",
      "Use RTK-only mode for command output savings.",
      "Keep model selection manual until compatibility checks are explicit.",
    ],
  },
  {
    id: "aider",
    name: "Aider",
    category: "agent",
    supportStatus: "planned",
    statusLabel: "Gated",
    setupPhase: "Adapt",
    integrationTarget:
      "Local environment/provider wrapper plus Repo Intelligence context packs.",
    notes:
      "Good fit for RTK and future repo graph context because it is frequently used inside long coding sessions.",
    capabilityBadges: ["CLI detection", "RTK-safe today", "Repo packs gated"],
    supportedModes: ["RTK only", "Repo packs", "Off"],
    safeToday:
      "Use RTK for noisy verification commands and copy implementation or handoff packs into Aider.",
    firstAutomation:
      "Add a reversible environment wrapper that points one Aider launch at local routing without editing saved secrets.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can surface a local Aider install.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail:
          "Use Repo Intelligence context packs alongside Aider prompts today.",
      },
      {
        label: "Provider wrapper",
        state: "Gated",
        detail:
          "Automatic provider environment wrapping waits for reversible setup state.",
      },
    ],
    configSurfaces: [
      "Aider binary",
      "provider environment",
      "repo context files",
    ],
    automationGates: [
      "Detect provider configuration without exposing secrets.",
      "Route through a reversible environment wrapper first.",
      "Expose Repo Intelligence packs without writing into the repo by default.",
    ],
    manualWorkflow: [
      "Confirm Aider is installed.",
      "Copy implementation or handoff packs into long Aider sessions.",
      "Use RTK-only mode for noisy verification commands.",
    ],
  },
  {
    id: "continue",
    name: "Continue",
    category: "editor",
    supportStatus: "planned",
    statusLabel: "Gated",
    setupPhase: "Guide",
    integrationTarget: "Local config adapter with explicit backup and restore.",
    notes:
      "Start with read-only detection and guided setup because Continue configs often contain multiple providers.",
    capabilityBadges: [
      "Config detection",
      "Guided setup",
      "Backup/restore gated",
    ],
    supportedModes: ["Repo packs", "Guided setup", "Off"],
    safeToday:
      "Guide users to review Continue provider config and copy Repo Intelligence packs manually.",
    firstAutomation:
      "Parse the provider list losslessly, back it up, and show an exact restore plan before writes.",
    capabilityRows: [
      {
        label: "Config detection",
        state: "Available now",
        detail: "Switchboard can guide users to the Continue config folder.",
      },
      {
        label: "Manual setup",
        state: "Manual today",
        detail:
          "Review configured providers before choosing any local proxy route.",
      },
      {
        label: "Config adapter",
        state: "Gated",
        detail:
          "Automatic edits wait for multi-provider backup and restore coverage.",
      },
    ],
    configSurfaces: [
      "Continue config folder",
      "provider list",
      "editor integration",
    ],
    automationGates: [
      "Parse multi-provider configs without dropping unknown fields.",
      "Back up the exact config before provider routing changes.",
      "Offer guided setup before automatic edits.",
    ],
    manualWorkflow: [
      "Open the Continue config folder.",
      "Review configured providers manually.",
      "Use Repo Intelligence packs beside Continue until the adapter can preserve every provider entry.",
    ],
  },
  {
    id: "goose",
    name: "Goose",
    category: "agent",
    supportStatus: "planned",
    statusLabel: "Gated",
    setupPhase: "Adapt",
    integrationTarget:
      "Local provider adapter and MCP/Repo Intelligence handoff.",
    notes:
      "Useful target once Switchboard has a stable connector capability model for agent-style tools.",
    capabilityBadges: [
      "CLI detection",
      "MCP handoff gated",
      "Repo packs gated",
    ],
    supportedModes: ["RTK only", "Repo packs", "Off"],
    safeToday:
      "Detect Goose and copy Repo Intelligence packs into sessions while MCP handoff remains gated.",
    firstAutomation:
      "Prototype a read-only MCP handoff manifest before managing provider configuration.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can check for a local Goose command.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail:
          "Copy Repo Intelligence packs into Goose sessions while adapter work lands.",
      },
      {
        label: "MCP handoff",
        state: "Gated",
        detail:
          "Automatic MCP and provider handoff waits for tested connector state.",
      },
    ],
    configSurfaces: ["Goose binary", "provider config", "MCP handoff"],
    automationGates: [
      "Detect Goose provider configuration safely.",
      "Confirm MCP handoff shape before adding managed setup.",
      "Verify Off mode removes local provider routing and leaves MCP config intact.",
    ],
    manualWorkflow: [
      "Confirm Goose is installed.",
      "Copy Repo Intelligence packs into Goose sessions today.",
      "Wait for managed MCP handoff before enabling automatic provider setup.",
    ],
  },
  {
    id: "qwen_code",
    name: "Qwen Code",
    category: "cli",
    supportStatus: "planned",
    statusLabel: "Gated",
    setupPhase: "Guide",
    integrationTarget:
      "CLI detection plus read-only Repo Intelligence handoff before reversible provider routing.",
    notes:
      "Treat as provider/account-sensitive coding CLI until model account compatibility can be checked without editing config.",
    capabilityBadges: [
      "CLI detection",
      "Repo packs today",
      "Provider routing gated",
    ],
    supportedModes: ["RTK only", "Repo packs", "Off"],
    safeToday:
      "Detect local Qwen Code command copy bounded Repo Intelligence packs into sessions.",
    firstAutomation:
      "Add read-only provider/model probe reversible environment wrapper before routing through Headroom.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can check qwen-code or qwen commands.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail:
          "Use Repo Intelligence implementation packs with Qwen Code today.",
      },
      {
        label: "Provider routing",
        state: "Gated",
        detail:
          "Automatic routing waits model/account guardrails restore coverage.",
      },
    ],
    configSurfaces: [
      "Qwen Code binary",
      "provider/model settings",
      "shell environment",
    ],
    automationGates: [
      "Detect stable Qwen Code command provider surface.",
      "Validate model/account compatibility before routing.",
      "Verify Off mode removes local proxy routing without touching account state.",
    ],
    manualWorkflow: [
      "Confirm Qwen Code installed.",
      "Copy Repo Intelligence implementation packs into Qwen Code sessions.",
      "Keep provider routing manual until Doctor can verify compatibility.",
    ],
  },
  {
    id: "amazon_q",
    name: "Amazon Q Developer CLI",
    category: "cli",
    supportStatus: "planned",
    statusLabel: "Gated",
    setupPhase: "Detect",
    integrationTarget:
      "Local CLI detection verification-pack handoff without changing AWS or provider state.",
    notes:
      "Keep AWS account credentials profile state outside managed switchboard storage.",
    capabilityBadges: [
      "CLI detection",
      "Repo packs today",
      "Credential-safe gated",
    ],
    supportedModes: ["RTK only", "Repo packs", "Off"],
    safeToday:
      "Detect Amazon Q CLI use verification packs build/test questions.",
    firstAutomation:
      "Add read-only provider status detection that never reads or stores AWS secrets.",
    capabilityRows: [
      {
        label: "Detection",
        state: "Available now",
        detail: "Switchboard can check whether q present on PATH.",
      },
      {
        label: "Repo context",
        state: "Manual today",
        detail:
          "Use Repo Intelligence verification packs Amazon Q Developer CLI.",
      },
      {
        label: "Credential guardrails",
        state: "Gated",
        detail:
          "Automatic setup waits AWS profile-safe detection restore policy.",
      },
    ],
    configSurfaces: [
      "Amazon Q CLI binary",
      "AWS profile state",
      "shell environment",
    ],
    automationGates: [
      "Detect q command without reading credentials.",
      "Keep AWS profile SSO state outside app storage.",
      "Prove Off mode does not alter AWS config or credentials.",
    ],
    manualWorkflow: [
      "Confirm Amazon Q Developer CLI is installed.",
      "Use Repo Intelligence verification packs build test work.",
      "Do not route provider traffic automatically until credential guardrails ship.",
    ],
  },
];

export const promotedSidecarConnectorIds = new Set([
  "cursor",
  "grok_cli",
  "aider",
  "continue",
  "goose",
  "qwen_code",
  "amazon_q",
]);

export const pendingPlannedConnectors: PlannedConnector[] =
  plannedConnectors.filter(
    (connector) => !promotedSidecarConnectorIds.has(connector.id),
  );

const plannedConnectorSafetyDossiers: Record<
  string,
  PlannedConnectorSafetyDossier
> = {
  gemini_cli: {
    connectorId: "gemini_cli",
    configPathStrategy:
      "Detect PATH: gemini first, then apply Switchboard-managed shell/base-url exports with sibling rollback backups.",
    providerSemantics:
      "Route Gemini CLI through the local Headroom-compatible base-url surface managed by Switchboard.",
    accountCaveat:
      "Model and account compatibility must stay visible; no account tokens are stored by Switchboard.",
    rollbackStrategy:
      "Restore the previous provider settings or remove only Switchboard-managed shell routing.",
  },
  opencode: {
    connectorId: "opencode",
    configPathStrategy:
      "Detect PATH: opencode, then identify the active provider config path before any write.",
    providerSemantics:
      "Provider config may be file-based or environment-based, so setup starts with a dry-run diff.",
    accountCaveat:
      "Secrets stay in the user's existing provider store and must not be copied into Switchboard state.",
    rollbackStrategy:
      "Restore the timestamped provider-config backup and clear managed environment overrides.",
  },
  cursor: {
    connectorId: "cursor",
    configPathStrategy:
      "Find the active Cursor app/profile settings surface before reading user settings.",
    providerSemantics:
      "Editor routing depends on profile and release-channel settings, not a single global base URL.",
    accountCaveat:
      "Account-specific model choices remain user-controlled until Doctor can explain compatibility.",
    rollbackStrategy:
      "Restore the exact profile settings backup without touching extension-managed secrets.",
  },
  grok_cli: {
    connectorId: "grok_cli",
    configPathStrategy:
      "Detect PATH: grok or PATH: xai and avoid guessing hidden provider files.",
    providerSemantics:
      "Only offer OpenAI-compatible routing after a stable xAI CLI provider surface is detected.",
    accountCaveat:
      "Unsupported model/account combinations require Doctor guardrails before setup is offered.",
    rollbackStrategy:
      "Remove managed shell routing and leave API key/account state outside app storage.",
  },
  aider: {
    connectorId: "aider",
    configPathStrategy:
      "Detect PATH: aider and prefer a one-launch environment wrapper over saved config edits.",
    providerSemantics:
      "Provider routing should be scoped to a reversible environment wrapper before persistent config support.",
    accountCaveat:
      "Existing provider secrets remain in the user's shell or provider config and are never copied.",
    rollbackStrategy:
      "Drop the wrapper environment and leave the user's Aider/provider files unchanged.",
  },
  continue: {
    connectorId: "continue",
    configPathStrategy:
      "Open or parse the Continue config folder only after preserving unknown provider fields.",
    providerSemantics:
      "Continue may contain multiple providers, so local routing must preserve every non-managed entry.",
    accountCaveat:
      "Provider credentials and account selections stay visible and user-owned during guided setup.",
    rollbackStrategy:
      "Restore the exact config backup or remove only the marked Switchboard provider entry.",
  },
  goose: {
    connectorId: "goose",
    configPathStrategy:
      "Detect PATH: goose and inspect Goose provider/MCP surfaces read-only before handoff.",
    providerSemantics:
      "Separate provider routing from MCP handoff so Repo Intelligence can stay read-only.",
    accountCaveat:
      "Provider account state remains outside Switchboard until compatibility checks are explicit.",
    rollbackStrategy:
      "Remove managed provider routing while preserving unrelated Goose MCP configuration.",
  },
  qwen_code: {
    connectorId: "qwen_code",
    configPathStrategy:
      "Detect PATH: qwen-code or PATH: qwen, then probe provider/model settings read-only.",
    providerSemantics:
      "Use Repo Intelligence handoff first; route provider traffic only after model guardrails exist.",
    accountCaveat:
      "Qwen account and model compatibility must be verified without editing config.",
    rollbackStrategy:
      "Remove managed shell routing and restore provider settings from the exact backup.",
  },
  amazon_q: {
    connectorId: "amazon_q",
    configPathStrategy:
      "Detect PATH: q and avoid reading AWS credentials, SSO caches, or profile secrets.",
    providerSemantics:
      "Treat Amazon Q as credential-sensitive; handoff packs are safe before provider routing.",
    accountCaveat:
      "AWS profile, SSO, and credential state must remain outside Switchboard storage.",
    rollbackStrategy:
      "Remove managed routing without modifying AWS config, credentials, SSO cache, or profiles.",
  },
  windsurf: {
    connectorId: "windsurf",
    configPathStrategy:
      "Detect the Windsurf app and active settings location before applying managed editor settings routing.",
    providerSemantics:
      "Manage only the Switchboard editor settings routing block while preserving unrelated Windsurf settings.",
    accountCaveat:
      "Switchboard preserves unrelated account and model settings while managing only its editor settings routing block.",
    rollbackStrategy:
      "Restore the active settings backup and remove only Switchboard-managed editor settings routing entries.",
  },
  zed_ai: {
    connectorId: "zed_ai",
    configPathStrategy:
      "Detect the Zed app settings file at ~/.config/zed/settings.json before applying managed assistant settings routing.",
    providerSemantics:
      "Assistant settings routing must preserve Zed assistant settings and any non-managed providers.",
    accountCaveat:
      "Switchboard preserves unrelated provider/account settings while managing only its local proxy routing entry.",
    rollbackStrategy:
      "Restore assistant settings from backup and remove only Switchboard-managed local proxy routing entries.",
  },
};

export function getPlannedConnector(id: string) {
  return (
    plannedConnectors.find((connector) => connector.id === id) ??
    managedConnectorDossiers.find((connector) => connector.id === id) ??
    null
  );
}

export function getPlannedConnectorSafetyDossier(
  id: string,
): PlannedConnectorSafetyDossier | null {
  return plannedConnectorSafetyDossiers[id] ?? null;
}

export function getPlannedConnectorSafetyDossiers(
  connectors: PlannedConnector[] = pendingPlannedConnectors,
) {
  return connectors.map((connector) => {
    const dossier = getPlannedConnectorSafetyDossier(connector.id);
    if (!dossier) {
      throw new Error(`Missing safety dossier for ${connector.id}.`);
    }
    return dossier;
  });
}

export function formatPlannedConnectorSafetyDossierMarkdown(
  connector: PlannedConnector,
) {
  const dossier = getPlannedConnectorSafetyDossier(connector.id);
  if (!dossier) {
    return "";
  }

  return [
    `## ${connector.name}`,
    `- Config paths: ${dossier.configPathStrategy}`,
    `- Provider/base-url semantics: ${dossier.providerSemantics}`,
    `- Account caveat: ${dossier.accountCaveat}`,
    `- Rollback strategy: ${dossier.rollbackStrategy}`,
  ].join("\n");
}

export function getPlannedConnectorConfigCreationPlan(
  connector: ConnectorDossier,
): PlannedConnectorConfigCreationPlan {
  const dossier = getPlannedConnectorSafetyDossier(connector.id);
  if (!dossier) {
    throw new Error(`Missing safety dossier for ${connector.id}.`);
  }

  const steps: PlannedConnectorConfigCreationStep[] = [
    {
      id: "detect",
      label: "Detect config surface",
      detail: dossier.configPathStrategy,
      requiredEvidence: [
        "Read-only binary or app detection result.",
        "Detected config, settings, profile, or environment surface documented without writes.",
      ],
    },
    {
      id: "dryRunDiff",
      label: "Show dry-run diff",
      detail:
        "Preview a copyable dry-run artifact with target path, before/after provider intent, managed marker boundary, rollback preview, and confirmation phrase before any file, profile, or environment edit.",
      requiredEvidence: [
        "User-visible dry-run diff artifact showing target, before/after local proxy/provider change, managed marker boundary, rollback preview, and confirmation phrase.",
        "No files, profiles, credentials, or account state changed by the preview.",
      ],
    },
    {
      id: "backup",
      label: "Create backup",
      detail:
        "Write a timestamped backup beside the edited config or record an environment-wrapper restore point.",
      requiredEvidence: [
        "Timestamped backup path or environment-wrapper restore point.",
        "Fixture-home restore test proving unknown fields and unrelated provider entries are preserved.",
      ],
    },
    {
      id: "apply",
      label: "Apply with consent",
      detail: dossier.providerSemantics,
      requiredEvidence: [
        "Explicit user consent captured for the connector and config surface.",
        "Managed marker or wrapper boundary proving only Switchboard-owned routing was applied.",
      ],
    },
    {
      id: "verify",
      label: "Verify in Doctor",
      detail: dossier.accountCaveat,
      requiredEvidence: [
        "Doctor check confirming account/model guardrails without storing secrets.",
        "Compatibility or caveat message visible before routing is considered supported.",
      ],
    },
    {
      id: "rollback",
      label: "Rollback safely",
      detail: dossier.rollbackStrategy,
      requiredEvidence: [
        "Fixture-home rollback test restoring the exact backup or removing only managed wrapper state.",
        "Post-rollback diff proving unrelated user settings are unchanged.",
      ],
    },
    {
      id: "offCleanup",
      label: "Clean up in Off mode",
      detail:
        "Off mode removes only Switchboard-managed routing and leaves unrelated user config untouched.",
      requiredEvidence: [
        "Fixture-home Off-mode cleanup showing managed routing removed.",
        "Doctor verification that the connector returns to manual or RTK-only mode.",
      ],
    },
  ];

  if (connector.supportStatus === "managed" && connector.setupPhase === "Managed") {
    return {
      connectorId: connector.id,
      connectorName: connector.name,
      automationEnabled: true,
      safetyNote:
        "Managed routing is enabled with backup, apply, verify, rollback, and Off cleanup evidence.",
      steps,
    };
  }

  return {
    connectorId: connector.id,
    connectorName: connector.name,
    automationEnabled: false,
    safetyNote:
      "Config creation remains gated until every step has tests and Doctor evidence.",
    steps,
  };
}

export function getPlannedConnectorConfigCreationPlans(
  connectors: ConnectorDossier[] = pendingPlannedConnectors,
) {
  return connectors.map(getPlannedConnectorConfigCreationPlan);
}

export function formatPlannedConnectorConfigCreationPlansMarkdown(
  connectors: ConnectorDossier[] = pendingPlannedConnectors,
) {
  const title =
    connectors.length === 1
      ? "# Mac AI Switchboard Connector Config Creation Plan"
      : "# Mac AI Switchboard Connector Config Creation Plans";

  return [
    title,
    "",
    "Automation stays disabled until detection, dry-run diff, backup, apply, verify, rollback, and Off cleanup are implemented and tested.",
    "",
    ...getPlannedConnectorConfigCreationPlans(connectors).flatMap((plan) => [
      `## ${plan.connectorName}`,
      `- Automation enabled: ${plan.automationEnabled ? "yes" : "no"}`,
      `- Safety note: ${plan.safetyNote}`,
      ...plan.steps.map(
        (step) =>
          `- ${step.label}: ${step.detail} Required evidence: ${step.requiredEvidence.join(" ")}`,
      ),
      "",
    ]),
  ]
    .join("\n")
    .trimEnd();
}

export function summarizePlannedConnectorSupport(
  connectors: ConnectorDossier[] = pendingPlannedConnectors,
): PlannedConnectorSupportSummary {
  const capabilityRows = connectors.flatMap((connector) =>
    connector.capabilityRows.map((capability) => ({
      connectorName: connector.name,
      ...capability,
    })),
  );
  const safeToday = capabilityRows.filter(
    (capability) => capability.state === "Available now",
  );
  const manualToday = capabilityRows.filter(
    (capability) => capability.state === "Manual today",
  );
  const gated = capabilityRows.filter(
    (capability) => capability.state === "Gated",
  );

  return {
    connectorCount: connectors.length,
    safeTodayCount: safeToday.length,
    manualTodayCount: manualToday.length,
    plannedCount: gated.length,
    automationGateCount: connectors.reduce(
      (total, connector) => total + connector.automationGates.length,
      0,
    ),
    safeTodayLabels: safeToday.map(
      (capability) => `${capability.connectorName}: ${capability.label}`,
    ),
    plannedLabels: gated.map(
      (capability) => `${capability.connectorName}: ${capability.label}`,
    ),
  };
}

export function getPlannedConnectorSetupGuide(
  id: string,
): PlannedConnectorSetupGuide | null {
  switch (id) {
    case "gemini_cli":
      return {
        label: "Check Gemini CLI",
        command: "command -v gemini && gemini --help",
        notes:
          "Use this only to confirm the CLI is present. Mac AI Switchboard will add provider routing once reversible Gemini config support lands.",
      };
    case "opencode":
      return {
        label: "Check OpenCode",
        command: "command -v opencode && opencode --help",
        notes:
          "Confirms the OpenCode binary before the app offers backed-up provider config edits.",
      };
    case "cursor":
      return {
        label: "Open Cursor settings",
        command: "open -a Cursor",
        notes:
          "Open Cursor and review model/provider settings manually. Automatic routing waits for account-safe settings detection.",
      };
    case "grok_cli":
      return {
        label: "Check xAI CLI",
        command: "command -v grok || command -v xai",
        notes:
          "Confirms whether a local xAI/Grok CLI exists. Provider/model compatibility remains a Doctor check before routing.",
      };
    case "aider":
      return {
        label: "Check Aider",
        command: "command -v aider && aider --help",
        notes:
          "Confirms Aider is available. RTK-only mode can already reduce noisy shell output while provider wrapping is built.",
      };
    case "continue":
      return {
        label: "Inspect Continue config",
        command: "open ~/.continue",
        notes:
          "Review configured providers manually. Mac AI Switchboard will only edit Continue once backup and restore coverage exists.",
      };
    case "goose":
      return {
        label: "Check Goose",
        command: "command -v goose && goose --help",
        notes:
          "Confirms Goose is present before local provider and MCP handoff support is enabled.",
      };
    case "qwen_code":
      return {
        label: "Check Qwen Code",
        command: "command -v qwen-code || command -v qwen",
        notes:
          "Confirms Qwen Code is present. Keep provider routing manual until model and account guardrails are explicit.",
      };
    case "amazon_q":
      return {
        label: "Check Amazon Q Developer CLI",
        command: "command -v q && q --version",
        notes:
          "Confirms the CLI is present without reading AWS credentials or changing profile state.",
      };
    case "windsurf":
      return {
        label: "Open Windsurf",
        command: "open -a Windsurf",
        notes:
          "Open Windsurf after enabling the connector to verify managed editor settings routing through Switchboard.",
      };
    case "zed_ai":
      return {
        label: "Open Zed",
        command: "open -a Zed",
        notes:
          "Open Zed after enabling the connector to verify managed assistant settings routing through Switchboard.",
      };
    default:
      return null;
  }
}

function readinessStage(
  id: PlannedConnectorReadinessStageId,
  label: string,
  state: PlannedConnectorReadinessStage["state"],
  evidence: string,
): PlannedConnectorReadinessStage {
  return { id, label, state, evidence };
}

export function getPlannedConnectorReadinessContract(
  connector: ConnectorDossier,
): PlannedConnectorReadinessContract {
  const setupGuide = getPlannedConnectorSetupGuide(connector.id);
  const isManagedConnector =
    connector.supportStatus === "managed" && connector.setupPhase === "Managed";
  const hasDetection = connector.capabilityRows.some(
    (capability) =>
      capability.label.toLowerCase().includes("detection") &&
      capability.state === "Available now",
  );
  const hasManualGuide =
    connector.manualWorkflow.length >= 3 && setupGuide !== null;
  const managedEvidence = (label: string) =>
    connector.capabilityRows.find(
      (capability) =>
        capability.label.toLowerCase().includes(label) &&
        capability.state === "Available now",
    )?.detail;

  const stages: PlannedConnectorReadinessStage[] = [
    readinessStage(
      "detected",
      "Detected",
      hasDetection || isManagedConnector ? "ready" : "blocked",
      hasDetection || isManagedConnector
        ? "Connector detection or managed setup evidence is available now."
        : "Add read-only detection before any setup path.",
    ),
    readinessStage(
      "manualGuide",
      "Manual Guide",
      hasManualGuide ? "ready" : "blocked",
      hasManualGuide
        ? setupGuide.notes
        : "Add a manual setup guide before automation is offered.",
    ),
    readinessStage(
      "backupImplemented",
      "Backup Implemented",
      isManagedConnector ? "ready" : "blocked",
      isManagedConnector
        ? "Managed setup creates a rollback backup before editing settings."
        : "No gated connector can write config until exact backup coverage exists.",
    ),
    readinessStage(
      "applyImplemented",
      "Apply Implemented",
      isManagedConnector ? "ready" : "blocked",
      managedEvidence("routing") ??
        "Automatic setup is disabled until a reversible apply path exists.",
    ),
    readinessStage(
      "verifyImplemented",
      "Verify Implemented",
      isManagedConnector ? "ready" : "blocked",
      managedEvidence("verification") ??
        "Doctor verification must prove the connector state after setup.",
    ),
    readinessStage(
      "rollbackImplemented",
      "Rollback Implemented",
      isManagedConnector ? "ready" : "blocked",
      managedEvidence("rollback") ??
        "Rollback must restore previous config without touching unrelated settings.",
    ),
    readinessStage(
      "offCleanupImplemented",
      "Off Cleanup Implemented",
      isManagedConnector ? "ready" : "blocked",
      isManagedConnector
        ? "Off mode removes only Switchboard-owned managed routing."
        : "Off mode cleanup must remove managed routing before automation is enabled.",
    ),
  ];
  const nextBlockedStage =
    stages.find((stage) => stage.state === "blocked")?.id ?? null;

  return {
    connectorId: connector.id,
    connectorName: connector.name,
    setupPhase: connector.setupPhase,
    automationEnabled: nextBlockedStage === null,
    nextBlockedStage,
    stages,
  };
}

export function getPlannedConnectorReadinessContracts(
  connectors: ConnectorDossier[] = plannedConnectors,
) {
  return connectors.map(getPlannedConnectorReadinessContract);
}

export function getPlannedConnectorReadinessBadges(
  connector: ConnectorDossier,
): PlannedConnectorReadinessBadge[] {
  const readiness = getPlannedConnectorReadinessContract(connector);
  const notes = [
    connector.notes,
    connector.safeToday,
    connector.firstAutomation,
    ...connector.configSurfaces,
    ...connector.automationGates,
    ...connector.manualWorkflow,
  ].join(" ");
  const hasAccountOrModelCaveat =
    /\b(account|model|credential|credentials|profile|AWS|SSO|secrets?)\b/i.test(
      notes,
    );

  const badges: PlannedConnectorReadinessBadge[] = [];
  if (
    connector.setupPhase === "Detect" ||
    connector.setupPhase === "Guide" ||
    connector.supportedModes.includes("Guided setup") ||
    connector.supportedModes.includes("Repo packs")
  ) {
    badges.push({
      kind: "manual-only",
      label: "Manual only",
      detail:
        "Safe today through detection, guided setup, RTK, or Repo Intelligence handoff without config writes.",
    });
  }

  if (!readiness.automationEnabled) {
    const nextStage = readiness.stages.find(
      (stage) => stage.id === readiness.nextBlockedStage,
    );
    badges.push({
      kind: "automation-gated",
      label: "Automation gated",
      detail: nextStage
        ? `Blocked until ${nextStage.label.toLowerCase()} is implemented.`
        : "Blocked until every readiness stage is implemented.",
    });
  } else {
    badges.push({
      kind: "verified-automation",
      label: "Verified automation",
      detail:
        "Backup, apply, verify, rollback, and Off cleanup coverage are complete.",
    });
  }

  if (hasAccountOrModelCaveat) {
    badges.push({
      kind: "unsupported-account-model",
      label: "Unsupported account/model",
      detail:
        "Account, credential, profile, or model compatibility needs Doctor guardrails before routing.",
    });
  }

  return badges;
}

export function getPlannedConnectorSetupChecklistScript() {
  const lines = [
    "# Mac AI Switchboard connector detection checks",
    "# Read-only: these commands only inspect local app/CLI availability.",
    ...plannedConnectors.flatMap((connector) => {
      const guide = getPlannedConnectorSetupGuide(connector.id);
      if (!guide) {
        return [];
      }
      return ["", `echo "== ${connector.name} =="`, `${guide.command} || true`];
    }),
  ];

  return lines.join("\n");
}
