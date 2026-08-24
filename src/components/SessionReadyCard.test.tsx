import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { SessionReadyCard } from "./SessionReadyCard";

describe("SessionReadyCard", () => {
  it("presents the session path and routes each step to its existing surface", () => {
    const setActiveView = vi.fn();
    render(<SessionReadyCard runtimeStatus={{ running: true, proxyReachable: true } as never} switchboardMode="full" setActiveView={setActiveView} />);

    expect(screen.getByRole("heading", { name: "Session Ready" })).toBeInTheDocument();
    expect(screen.getByText("Loopback route is healthy")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Repo context/i }));
    expect(setActiveView).toHaveBeenCalledWith("repoIntelligence");
    fireEvent.click(screen.getByRole("button", { name: /Harness \/ CLI/i }));
    expect(setActiveView).toHaveBeenCalledWith("workbench");
    fireEvent.click(screen.getByRole("button", { name: /Prepare agent handoff/i }));
    expect(setActiveView).toHaveBeenCalledWith("optimization");
  });

  it("makes an unavailable runtime actionable without claiming readiness", () => {
    const setActiveView = vi.fn();
    render(<SessionReadyCard runtimeStatus={null} switchboardMode="off" setActiveView={setActiveView} />);
    expect(screen.getByText("Run Doctor before relying on routing")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Runtime/i }));
    expect(setActiveView).toHaveBeenCalledWith("doctor");
  });
});
