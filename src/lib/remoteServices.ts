export interface RemoteServicesCopy {
  label: string;
  detail: string;
}

export function remoteServicesCopy(
  remoteServicesEnabled: boolean,
): RemoteServicesCopy {
  if (remoteServicesEnabled) {
    return {
      label: "Available",
      detail: "Account features and optional remote telemetry are enabled.",
    };
  }

  return {
    label: "Off",
    detail: "No pricing, trial, Clarity, Sentry, or Aptabase calls.",
  };
}

export function localOnlySetupLabel(localOnly: boolean): string {
  return localOnly ? "Local-only Mac setup" : "Headroom cloud setup";
}
