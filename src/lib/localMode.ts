const truthyValues = new Set(["1", "true", "yes", "on"]);

function truthy(value: unknown): boolean {
  return typeof value === "string" && truthyValues.has(value.trim().toLowerCase());
}

function localFreeBuildFlavor(value: unknown): boolean {
  return typeof value === "string" && value.trim().toLowerCase() === "local-free";
}

export function localOnlyModeEnabled(): boolean {
  if (truthy(import.meta.env.VITE_HEADROOM_LOCAL_ONLY)) {
    return true;
  }
  if (localFreeBuildFlavor(import.meta.env.VITE_HEADROOM_BUILD_FLAVOR)) {
    return true;
  }
  return !truthy(import.meta.env.VITE_HEADROOM_REMOTE_SERVICES);
}

export function remoteTelemetryEnabled(): boolean {
  return !localOnlyModeEnabled() && truthy(import.meta.env.VITE_HEADROOM_REMOTE_TELEMETRY);
}
