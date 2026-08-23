import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { loadTransportObservations } from "./transportObservations";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("transport observations", () => {
  it("loads the read-only bounded diagnostic command", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([]);
    await expect(loadTransportObservations()).resolves.toEqual([]);
    expect(invoke).toHaveBeenCalledWith("get_transport_observations");
  });
});

