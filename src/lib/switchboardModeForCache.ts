import type { RuntimeStatus } from "./types";
import type { SwitchboardModeId } from "./exactCacheDefaultPolicy";

/** Maps runtime status to the cache-policy mode bucket. */
export function resolveSwitchboardModeForCache(
  runtimeStatus: RuntimeStatus | null | undefined,
): SwitchboardModeId {
  if (!runtimeStatus?.running) {
    return "off";
  }
  if (runtimeStatus.rtk?.enabled && !runtimeStatus.proxyReachable) {
    return "rtk";
  }
  if (runtimeStatus.proxyReachable) {
    return runtimeStatus.rtk?.enabled ? "headroom" : "full";
  }
  return "off";
}
