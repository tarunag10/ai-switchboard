type TauriInternals = {
  transformCallback?: unknown;
  metadata?: {
    currentWindow?: {
      label?: unknown;
    };
  };
};

const getTauriInternals = (): TauriInternals | null => {
  const tauriInternals =
    typeof window === "undefined"
      ? null
      : (window as Window & { __TAURI_INTERNALS__?: unknown })
          .__TAURI_INTERNALS__;

  return typeof tauriInternals === "object" && tauriInternals !== null
    ? (tauriInternals as TauriInternals)
    : null;
};

/**
 * Window APIs in @tauri-apps/api assume that metadata.currentWindow exists.
 * A browser preview (including Vercel) can expose a placeholder
 * __TAURI_INTERNALS__ object without that metadata, so treating any object as
 * a desktop runtime causes getCurrentWindow() to throw during startup. Keep
 * this check aligned with the minimum shape required by the window API.
 */
export const hasTauriRuntime = () => {
  const internals = getTauriInternals();
  return typeof internals?.metadata?.currentWindow?.label === "string";
};

export const hasTauriEventRuntime = () =>
  typeof getTauriInternals()?.transformCallback === "function";
