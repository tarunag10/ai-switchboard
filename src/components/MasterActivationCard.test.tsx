import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { MasterActivationCard, type MasterFeatureId } from "./MasterActivationCard";

const featureIds: MasterFeatureId[] = ["agent-memory", "token-xray", "daily-briefing", "agent-session", "repo-intelligence", "addons", "gateway-mcp", "doctor", "rollback"];

describe("MasterActivationCard", () => {
  it("renders the master action and every independently actionable feature", () => {
    render(<MasterActivationCard onActivateAll={vi.fn()} onActivateFeature={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Activate everything" })).toBeInTheDocument();
    for (const label of ["Agent Memory", "Token X-Ray", "Daily Briefing", "Agent Session", "Repo Intelligence", "Add-ons", "Gateway / MCP", "Doctor", "Rollback inventory"]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.getByText(/Gateway\/MCP setup, provider credentials/)).toBeInTheDocument();
  });

  it("routes the master and row actions through callbacks", async () => {
    const user = userEvent.setup();
    const onActivateAll = vi.fn();
    const onDeactivateAll = vi.fn();
    const onActivateFeature = vi.fn();
    const onDeactivateFeature = vi.fn();
    const onOpenFeature = vi.fn();
    render(<MasterActivationCard onActivateAll={onActivateAll} onDeactivateAll={onDeactivateAll} onActivateFeature={onActivateFeature} onDeactivateFeature={onDeactivateFeature} onOpenFeature={onOpenFeature} />);

    await user.click(screen.getByRole("button", { name: "Activate everything" }));
    await user.click(screen.getByRole("button", { name: "Activate Agent Memory" }));
    await user.click(screen.getByRole("button", { name: "Open Agent Memory" }));

    expect(onActivateAll).toHaveBeenCalledOnce();
    expect(onActivateFeature).toHaveBeenCalledWith(featureIds[0]);
    expect(onOpenFeature).toHaveBeenCalledWith(featureIds[0]);
  });

  it("switches the master and completed feature actions to deactivation", async () => {
    const user = userEvent.setup();
    const onDeactivateAll = vi.fn();
    const onDeactivateFeature = vi.fn();
    render(<MasterActivationCard activationState="complete" progress={{ completed: 9, total: 9 }} featureStates={{ "agent-memory": { status: "complete" } }} onActivateAll={vi.fn()} onDeactivateAll={onDeactivateAll} onActivateFeature={vi.fn()} onDeactivateFeature={onDeactivateFeature} />);

    expect(screen.getByRole("button", { name: "Deactivate local workspace" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Deactivate Agent Memory" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "Deactivate local workspace" }));
    await user.click(screen.getByRole("button", { name: "Deactivate Agent Memory" }));
    expect(onDeactivateAll).toHaveBeenCalledOnce();
    expect(onDeactivateFeature).toHaveBeenCalledWith("agent-memory");
  });

  it("communicates running, partial, gated, and complete states accessibly", () => {
    const { rerender } = render(<MasterActivationCard activationState="running" progress={{ completed: 3, total: 9 }} onActivateAll={vi.fn()} onActivateFeature={vi.fn()} />);
    expect(screen.getByRole("button", { name: "Activating workspace…" })).toHaveAttribute("aria-busy", "true");
    expect(screen.getByText("3/9")).toBeInTheDocument();

    rerender(<MasterActivationCard activationState="partial" progress={{ completed: 7, total: 9 }} featureStates={{ "gateway-mcp": { status: "gated", detail: "Requires a user-owned gateway" } }} onActivateAll={vi.fn()} onActivateFeature={vi.fn()} />);
    expect(screen.getByText(/Activation needs a follow-up/)).toBeInTheDocument();
    expect(screen.getAllByText(/Requires a user-owned gateway/)).toHaveLength(2);

    rerender(<MasterActivationCard activationState="complete" progress={{ completed: 9, total: 9 }} onActivateAll={vi.fn()} onActivateFeature={vi.fn()} />);
    expect(screen.getByRole("button", { name: "Deactivate local workspace" })).toBeDisabled();
    expect(screen.getByText("All local features activated")).toBeInTheDocument();
  });

  it("disables conflicting controls and announces immediate activation progress", () => {
    render(
      <MasterActivationCard
        activationState="running"
        operation="activate"
        featureStates={{
          "agent-memory": {
            status: "running",
            detail: "Waiting for activation evidence.",
          },
        }}
        onActivateAll={vi.fn()}
        onActivateFeature={vi.fn()}
        onOpenFeature={vi.fn()}
        onActivateMaxCompression={vi.fn()}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent("Activation started");
    expect(
      screen.getByRole("button", { name: "Activating workspace…" }),
    ).toBeDisabled();
    expect(screen.getByRole("button", { name: "Activate Agent Memory" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Open Agent Memory" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Enable max compression" })).toBeDisabled();
  });

  it("surfaces the failed step and offers an accessible activation retry", async () => {
    const user = userEvent.setup();
    const onActivateAll = vi.fn(async () => undefined);
    render(
      <MasterActivationCard
        activationState="error"
        featureStates={{
          doctor: {
            status: "error",
            detail: "The local runtime did not start.",
            actionLabel: "Retry",
          },
        }}
        onActivateAll={onActivateAll}
        onActivateFeature={vi.fn()}
      />,
    );

    const status = screen.getByRole("status");
    expect(status).toHaveAttribute("aria-live", "assertive");
    expect(status).toHaveTextContent("The local runtime did not start");
    const retry = screen.getByRole("button", { name: "Retry activation" });
    expect(retry).toBeEnabled();
    await user.click(retry);
    expect(onActivateAll).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: "Retry Doctor" })).toBeEnabled();
  });

  it("treats completed activation as active even without an ownership receipt", () => {
    render(
      <MasterActivationCard
        activationState="complete"
        isActive={false}
        onActivateAll={vi.fn()}
        onDeactivateAll={vi.fn()}
        onActivateFeature={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("button", { name: "Deactivate local workspace" }),
    ).toBeEnabled();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Activation completed",
    );
  });
});
