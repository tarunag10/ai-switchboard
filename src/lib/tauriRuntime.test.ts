import { beforeEach, describe, expect, it } from "vitest";

import { hasTauriEventRuntime, hasTauriRuntime } from "./tauriRuntime";

const setTauriInternals = (value: unknown) => {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value,
  });
};

describe("tauriRuntime", () => {
  beforeEach(() => {
    delete (window as Window & { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
  });

  it("treats missing or placeholder internals as browser mode", () => {
    expect(hasTauriRuntime()).toBe(false);
    expect(hasTauriEventRuntime()).toBe(false);

    setTauriInternals(undefined);

    expect(hasTauriRuntime()).toBe(false);
    expect(hasTauriEventRuntime()).toBe(false);
  });

  it("separates window APIs from event listener support", () => {
    setTauriInternals({});

    expect(hasTauriRuntime()).toBe(true);
    expect(hasTauriEventRuntime()).toBe(false);

    setTauriInternals({ transformCallback: () => undefined });

    expect(hasTauriRuntime()).toBe(true);
    expect(hasTauriEventRuntime()).toBe(true);
  });
});
