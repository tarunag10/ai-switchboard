import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { GatewayProfilesCard } from "./GatewayProfilesCard";

describe("GatewayProfilesCard", () => {
  it("keeps remote disclosure and Doctor evidence behind an explicit details action", async () => {
    render(<GatewayProfilesCard onCopyGuidance={vi.fn()} />);
    const user = userEvent.setup();

    expect(screen.getByText("Semantic Cache")).toBeVisible();
    expect(screen.getByText("Cloudflare AI Gateway")).toBeVisible();
    expect(screen.queryByText(/requests pass through Cloudflare/i)).not.toBeVisible();

    await user.click(screen.getAllByRole("button", { name: "View privacy & Doctor" })[2]);

    expect(screen.getByText(/requests pass through Cloudflare/i)).toBeVisible();
    expect(screen.getAllByText("Required evidence")[2]).toBeVisible();
    expect(screen.getAllByText("Doctor checks")[2]).toBeVisible();
    expect(screen.getByText(/Switchboard writes nothing/i)).toBeVisible();
  });

  it("copies a manual guide without applying configuration", async () => {
    const onCopyGuidance = vi.fn();
    render(<GatewayProfilesCard onCopyGuidance={onCopyGuidance} />);
    const user = userEvent.setup();

    await user.click(screen.getAllByRole("button", { name: "Copy setup & Doctor guide" })[0]);

    expect(onCopyGuidance).toHaveBeenCalledWith(
      expect.stringContaining("Semantic Cache"),
      "Semantic Cache setup and Doctor guide",
    );
  });
});
