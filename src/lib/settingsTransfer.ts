import type {
  ClientConnectorStatus,
  DashboardState,
  ManagedTool,
  SavingsMode,
  SwitchboardMode,
} from "./types";

export const SETTINGS_TRANSFER_SCHEMA_VERSION = 1;

export interface SettingsExportBundle {
  schemaVersion: number;
  exportedAt: string;
  appVersion: string;
  preferences: {
    switchboardMode: SwitchboardMode;
    savingsMode: SavingsMode;
  };
  connectors: Array<{
    clientId: string;
    enabled: boolean;
    supportStatus: ClientConnectorStatus["supportStatus"];
  }>;
  addons: Array<{
    id: string;
    enabled: boolean;
    status: ManagedTool["status"];
    runtime: ManagedTool["runtime"];
  }>;
  caveats: string[];
}

export interface SettingsImportPreview {
  valid: boolean;
  title: string;
  detail: string;
  safePreferences: Partial<SettingsExportBundle["preferences"]>;
  manualItems: string[];
  errors: string[];
}

const caveats = [
  "No provider API keys, account emails, billing state, message logs, local paths, Keychain values, or token history are exported.",
  "Connector and add-on entries are advisory; import only applies safe app preferences.",
  "Run Doctor after importing settings on another Mac before enabling managed connector writes.",
];

function isSwitchboardMode(value: unknown): value is SwitchboardMode {
  return value === "off" || value === "rtk" || value === "headroom" || value === "full";
}

function isSavingsMode(value: unknown): value is SavingsMode {
  return value === "balanced" || value === "aggressive";
}

function safeConnector(connector: ClientConnectorStatus) {
  return {
    clientId: connector.clientId,
    enabled: connector.enabled,
    supportStatus: connector.supportStatus,
  };
}

function safeAddon(tool: ManagedTool) {
  return {
    id: tool.id,
    enabled: tool.enabled,
    status: tool.status,
    runtime: tool.runtime,
  };
}

export function buildSettingsExportBundle({
  dashboard,
  connectors,
  switchboardMode,
  savingsMode,
  exportedAt = new Date().toISOString(),
}: {
  dashboard: DashboardState;
  connectors: ClientConnectorStatus[];
  switchboardMode: SwitchboardMode;
  savingsMode: SavingsMode;
  exportedAt?: string;
}): SettingsExportBundle {
  return {
    schemaVersion: SETTINGS_TRANSFER_SCHEMA_VERSION,
    exportedAt,
    appVersion: dashboard.appVersion,
    preferences: {
      switchboardMode,
      savingsMode,
    },
    connectors: connectors
      .map(safeConnector)
      .sort((left, right) => left.clientId.localeCompare(right.clientId)),
    addons: dashboard.tools
      .filter((tool) => !tool.required)
      .map(safeAddon)
      .sort((left, right) => left.id.localeCompare(right.id)),
    caveats,
  };
}

export function formatSettingsExportBundle(bundle: SettingsExportBundle) {
  return JSON.stringify(bundle, null, 2);
}

export function parseSettingsImport(text: string): SettingsImportPreview {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return {
      valid: false,
      title: "Settings import is not valid JSON",
      detail: "Paste a Mac AI Switchboard settings export JSON bundle.",
      safePreferences: {},
      manualItems: [],
      errors: ["JSON parse failed."],
    };
  }

  if (!parsed || typeof parsed !== "object") {
    return {
      valid: false,
      title: "Settings import is not an object",
      detail: "The import bundle must be a JSON object.",
      safePreferences: {},
      manualItems: [],
      errors: ["Expected a JSON object."],
    };
  }

  const bundle = parsed as Partial<SettingsExportBundle>;
  const errors: string[] = [];
  if (bundle.schemaVersion !== SETTINGS_TRANSFER_SCHEMA_VERSION) {
    errors.push(`Unsupported schema version: ${String(bundle.schemaVersion)}.`);
  }

  const preferences =
    bundle.preferences && typeof bundle.preferences === "object"
      ? (bundle.preferences as Partial<SettingsExportBundle["preferences"]>)
      : {};
  const safePreferences: Partial<SettingsExportBundle["preferences"]> = {};
  if (isSwitchboardMode(preferences.switchboardMode)) {
    safePreferences.switchboardMode = preferences.switchboardMode;
  } else {
    errors.push("Missing or invalid switchboard mode.");
  }

  if (isSavingsMode(preferences.savingsMode)) {
    safePreferences.savingsMode = preferences.savingsMode;
  } else {
    errors.push("Missing or invalid savings profile.");
  }

  const connectors = Array.isArray(bundle.connectors) ? bundle.connectors : [];
  const addons = Array.isArray(bundle.addons) ? bundle.addons : [];
  const manualItems = [
    ...connectors.map((item) => {
      const connector = item as { clientId?: unknown; enabled?: unknown };
      return `Connector ${String(connector.clientId ?? "unknown")}: ${
        connector.enabled ? "enabled" : "disabled"
      } in export; review manually before applying config.`;
    }),
    ...addons.map((item) => {
      const addon = item as { id?: unknown; enabled?: unknown };
      return `Add-on ${String(addon.id ?? "unknown")}: ${
        addon.enabled ? "enabled" : "disabled"
      } in export; install or enable from Addons if wanted.`;
    }),
  ];

  const valid = errors.length === 0;
  return {
    valid,
    title: valid ? "Settings import ready" : "Settings import needs review",
    detail: valid
      ? "Safe app preferences can be applied. Connector and add-on entries remain manual."
      : "Fix the bundle before applying settings.",
    safePreferences,
    manualItems,
    errors,
  };
}
