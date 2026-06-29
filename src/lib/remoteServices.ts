export interface RemoteServicesCopy {
  label: string;
  detail: string;
}

export type RemoteServiceKind =
  "account" | "pricing" | "telemetry" | "analytics" | "updates" | "support";

export interface RemoteServiceDestination {
  id: string;
  kind: RemoteServiceKind;
  label: string;
  endpointExample: string;
  source: string;
  localOnlyAllowed: boolean;
}

export const remoteServiceDestinations: RemoteServiceDestination[] = [
  {
    id: "headroom_account_api",
    kind: "account",
    label: "Mac AI Switchboard account API",
    endpointExample: "configured account API host",
    source: "sign-in and account profile requests",
    localOnlyAllowed: false,
  },
  {
    id: "headroom_pricing_api",
    kind: "pricing",
    label: "Mac AI Switchboard pricing and trial API",
    endpointExample: "configured account API host",
    source: "pricing, trial, usage, and upgrade requests",
    localOnlyAllowed: false,
  },
  {
    id: "sentry",
    kind: "telemetry",
    label: "Sentry diagnostics",
    endpointExample: "configured DSN host",
    source: "error and crash diagnostics",
    localOnlyAllowed: false,
  },
  {
    id: "clarity",
    kind: "analytics",
    label: "Microsoft Clarity analytics",
    endpointExample: "configured session analytics host",
    source: "optional product analytics",
    localOnlyAllowed: false,
  },
  {
    id: "product_analytics",
    kind: "analytics",
    label: "Product analytics",
    endpointExample: "configured event analytics host",
    source: "optional product analytics",
    localOnlyAllowed: false,
  },
  {
    id: "tauri_updater",
    kind: "updates",
    label: "Tauri update feed",
    endpointExample:
      "configured signed update feed",
    source: "signed app update checks",
    localOnlyAllowed: false,
  },
  {
    id: "support_links",
    kind: "support",
    label: "External support links",
    endpointExample: "user-opened support destination",
    source: "user-opened repository support links",
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
  const paused = blockedLocalOnlyDestinations()
    .map((destination) => destination.label)
    .join(", ");
  return {
    label: "Local-only",
    detail: `Mac AI Switchboard local-only mode is on. Cloud account, pricing, diagnostics, analytics, update, and support endpoints stay paused: ${paused}.`,
  };
}

export function localOnlySetupLabel(localOnly: boolean): string {
  return localOnly ? "Local-only Mac setup" : "Mac AI Switchboard cloud setup";
}
