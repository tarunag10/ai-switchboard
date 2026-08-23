import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TransportObservationsPanel } from "./TransportObservationsPanel";

const { load } = vi.hoisted(() => ({ load: vi.fn() }));

vi.mock("../lib/transportObservations", () => ({
  loadTransportObservations: load,
}));

describe("TransportObservationsPanel", () => {
  beforeEach(() => {
    load.mockReset();
  });

  it("shows bounded redacted route and outcome telemetry", async () => {
    load.mockResolvedValue([
      {
        eventId: "event-1",
        startedAtMs: 100,
        completedAtMs: 180,
        route: "headroom",
        requestClass: "/v1/responses",
        streaming: true,
        statusCode: 200,
        terminalOutcome: "success",
      },
    ]);

    render(<TransportObservationsPanel />);

    expect(await screen.findByText("Headroom")).toBeInTheDocument();
    expect(screen.getByText("Success")).toBeInTheDocument();
    expect(screen.getByText("80 ms · HTTP 200")).toBeInTheDocument();
    expect(screen.getByText(/Content-free local route/)).toBeInTheDocument();
  });

  it("refreshes and surfaces loader failures", async () => {
    load.mockRejectedValueOnce(new Error("backend unavailable"));
    render(<TransportObservationsPanel />);

    expect(await screen.findByRole("alert")).toHaveTextContent("backend unavailable");
    load.mockResolvedValueOnce([]);
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(load).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("No transport observations yet.")).toBeInTheDocument();
  });
});
