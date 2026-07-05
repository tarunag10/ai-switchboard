import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SettingsOpenLoginCard } from "./SettingsOpenLoginCard";

describe("SettingsOpenLoginCard", () => {
  it("renders the current enabled state", () => {
    render(
      <SettingsOpenLoginCard
        autostartBusy={false}
        autostartEnabled
        onToggle={vi.fn()}
      />,
    );

    const toggle = screen.getByRole("switch", {
      name: "Disable open on login",
    });
    expect(toggle).toHaveAttribute("aria-checked", "true");
    expect(toggle).toHaveClass("is-on");
  });

  it("calls onToggle with the next state", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();

    render(
      <SettingsOpenLoginCard
        autostartBusy={false}
        autostartEnabled={false}
        onToggle={onToggle}
      />,
    );

    await user.click(
      screen.getByRole("switch", { name: "Enable open on login" }),
    );

    expect(onToggle).toHaveBeenCalledTimes(1);
    expect(onToggle).toHaveBeenCalledWith(true);
  });

  it("disables the switch while state is busy or unknown", () => {
    const { rerender } = render(
      <SettingsOpenLoginCard
        autostartBusy
        autostartEnabled={false}
        onToggle={vi.fn()}
      />,
    );

    expect(screen.getByRole("switch")).toBeDisabled();

    rerender(
      <SettingsOpenLoginCard
        autostartBusy={false}
        autostartEnabled={null}
        onToggle={vi.fn()}
      />,
    );

    expect(screen.getByRole("switch")).toBeDisabled();
  });
});
