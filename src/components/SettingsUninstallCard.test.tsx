import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { SettingsUninstallCard } from "./SettingsUninstallCard";

describe("SettingsUninstallCard", () => {
  it("renders uninstall copy and opens the uninstall dialog", () => {
    const onOpenUninstallDialog = vi.fn();

    render(
      <SettingsUninstallCard onOpenUninstallDialog={onOpenUninstallDialog} />,
    );

    expect(
      screen.getByRole("heading", { name: "Uninstall" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/runtime storage/i)).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: /uninstall ai switchboard/i }),
    );

    expect(onOpenUninstallDialog).toHaveBeenCalledTimes(1);
  });
});
