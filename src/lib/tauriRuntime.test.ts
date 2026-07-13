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

  it("requires window metadata before using window APIs", () => {
    setTauriInternals({});

    expect(hasTauriRuntime()).toBe(false);
    expect(hasTauriEventRuntime()).toBe(false);

    setTauriInternals({
      metadata: { currentWindow: { label: "main" } },
    });

    expect(hasTauriRuntime()).toBe(true);
    expect(hasTauriEventRuntime()).toBe(false);

    setTauriInternals({
      metadata: { currentWindow: { label: "main" } },
      transformCallback: () => undefined,
    });

    expect(hasTauriRuntime()).toBe(true);
    expect(hasTauriEventRuntime()).toBe(true);
  });

  it("does not enable event APIs for a partial browser shim", () => {
    setTauriInternals({ transformCallback: () => undefined });

    expect(hasTauriRuntime()).toBe(false);
    expect(hasTauriEventRuntime()).toBe(false);
  });
});
