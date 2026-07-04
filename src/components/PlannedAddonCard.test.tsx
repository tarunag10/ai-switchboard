import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { PlannedAddonCard } from "./PlannedAddonCard";
import type { PlannedAddon } from "../lib/plannedAddons";

const addon: PlannedAddon = {
  id: "rtk_hardening",
  name: "RTK Hardening",
  statusLabel: "Ready to harden",
  description: "Token-optimized command-output layer.",
  bullets: ["Keep install and disable flows reversible."],
  healthChecks: ["Recent RTK activity can be loaded from Addons card."],
  savingsSources: ["Command output summarized before it reaches agents."],
  verificationCommand: "npm run smoke:preflight",
};

describe("PlannedAddonCard", () => {
  it("keeps technical evidence collapsed behind an accessible details button", async () => {
    render(<PlannedAddonCard addon={addon} />);
    const user = userEvent.setup();

    expect(screen.getByText("Token-optimized command-output layer.")).toBeVisible();
    expect(
      screen.getByText("Keep install and disable flows reversible."),
    ).toBeVisible();

    const detailsButton = screen.getByRole("button", { name: "Learn more" });
    expect(detailsButton).toHaveAttribute("aria-expanded", "false");
    expect(detailsButton).toHaveAttribute("aria-controls");

    const detailsId = detailsButton.getAttribute("aria-controls");
    const details = document.getElementById(detailsId ?? "");
    expect(details).not.toBeNull();
    expect(details).not.toBeVisible();
    expect(screen.getByText("Health checks")).not.toBeVisible();
    expect(screen.getByText("npm run smoke:preflight")).not.toBeVisible();

    await user.click(detailsButton);

    expect(
      screen.getByRole("button", { name: "Hide details" }),
    ).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Health checks")).toBeVisible();
    expect(screen.getByText("Savings sources")).toBeVisible();
    expect(screen.getByText("npm run smoke:preflight")).toBeVisible();
  });

  it("keeps the Repo Intelligence action available while details are closed", () => {
    const onOpenRepoIntelligence = vi.fn();

    render(
      <PlannedAddonCard
        addon={{ ...addon, id: "repo_intelligence", name: "Repo Intelligence" }}
        onOpenRepoIntelligence={onOpenRepoIntelligence}
      />,
    );

    expect(
      screen.getByRole("button", { name: "Open Repo Intelligence" }),
    ).toBeVisible();
    expect(screen.getByText("npm run smoke:preflight")).not.toBeVisible();
  });
});
