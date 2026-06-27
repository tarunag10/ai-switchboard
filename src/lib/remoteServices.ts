export interface RemoteServicesCopy {
  label: string;
  detail: string;
}

export type RemoteServiceKind =
  | "account"
  | "pricing"
  | "telemetry"
  | "analytics"
  | "updates"
  | "support";

export interface RemoteServiceDestination {
  id: string;
  kind: RemoteServiceKind;
  label: string;
  envVar?: string;
  endpointExample: string;
  source: string;
  localOnlyAllowed: boolean;
}

export const remoteServiceDestinations: RemoteServiceDestination[] = [
  {
    id: "headroom_account_api",
    kind: "account",
    label: "Headroom account API",
    envVar: "HEADROOM_ACCOUNT_API_BASE_URL",
    endpointExample: "https://extraheadroom.com/api/v1",
    source: "sign-in and account profile requests",
    localOnlyAllowed: false,
  },
  {
    id: "headroom_pricing_api",
    kind: "pricing",
    label: "Headroom pricing and trial API",
    envVar: "HEADROOM_ACCOUNT_API_BASE_URL",
    endpointExample: "https://extraheadroom.com/api/v1",
    source: "pricing, trial, usage, and upgrade requests",
    localOnlyAllowed: false,
  },
  {
    id: "sentry",
    kind: "telemetry",
    label: "Sentry diagnostics",
    envVar: "SENTRY_DSN",
    endpointExample: "configured DSN host",
    source: "error and crash diagnostics",
    localOnlyAllowed: false,
  },
  {
    id: "clarity",
    kind: "analytics",
    label: "Microsoft Clarity analytics",
    envVar: "VITE_CLARITY_PROJECT_ID",
    endpointExample: "https://www.clarity.ms",
    source: "optional product analytics",
    localOnlyAllowed: false,
  },
  {
    id: "aptabase",
    kind: "analytics",
    label: "Aptabase analytics",
    envVar: "HEADROOM_APTABASE_APP_KEY",
    endpointExample: "https://app.aptabase.com",
    source: "optional product analytics",
    localOnlyAllowed: false,
  },
  {
    id: "tauri_updater",
    kind: "updates",
    label: "Tauri update feed",
    envVar: "HEADROOM_UPDATER_ENDPOINTS",
    endpointExample:
      "https://github.com/tarunag10/mac-ai-switchboard/releases/latest/download/latest.json",
    source: "signed app update checks",
    localOnlyAllowed: false,
  },
  {
    id: "support_links",
    kind: "support",
    label: "External support links",
    endpointExample: "mailto:support@extraheadroom.com",
    source: "user-opened support links",
    localOnlyAllowed: false,
  },
];

export function blockedLocalOnlyDestinations(): RemoteServiceDestination[] {
  return remoteServiceDestinations.filter(
    (destination) => !destination.localOnlyAllowed,
  );
}

export function allowedRemoteDestinations(
  localOnly: boolean,
): RemoteServiceDestination[] {
  if (localOnly) {
    return remoteServiceDestinations.filter(
      (destination) => destination.localOnlyAllowed,
    );
  }
  return remoteServiceDestinations;
}

export function remoteServicesCopy(
  remoteServicesEnabled: boolean,
): RemoteServicesCopy {
  if (remoteServicesEnabled) {
    return {
      label: "Available",
      detail:
        "Account, pricing, update, support, and optional telemetry destinations are enabled.",
    };
  }
  const blocked = blockedLocalOnlyDestinations()
    .map((destination) => destination.label)
    .join(", ");
  return {
    label: "Off",
    detail: `Blocked in local-only mode: ${blocked}.`,
  };
}

export function localOnlySetupLabel(localOnly: boolean): string {
  return localOnly ? "Local-only Mac setup" : "Headroom cloud setup";
}
