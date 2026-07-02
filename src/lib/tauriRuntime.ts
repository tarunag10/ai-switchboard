type TauriInternals = {
  transformCallback?: unknown;
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

export const hasTauriRuntime = () => getTauriInternals() !== null;

export const hasTauriEventRuntime = () =>
  typeof getTauriInternals()?.transformCallback === "function";
