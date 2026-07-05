import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SettingsUninstallCard } from "./SettingsUninstallCard";

describe("SettingsUninstallCard", () => {
  it("renders uninstall copy and opens the uninstall dialog", async () => {
    const user = userEvent.setup();
    const onOpenUninstallDialog = vi.fn();

    render(
      <SettingsUninstallCard onOpenUninstallDialog={onOpenUninstallDialog} />,
    );

    expect(
      screen.getByRole("heading", { name: "Uninstall" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/runtime storage/i)).toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: /uninstall ai switchboard/i }),
    );

    expect(onOpenUninstallDialog).toHaveBeenCalledTimes(1);
  });
});
