import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { TraySidebar } from "./TraySidebar";

describe("TraySidebar", () => {
  it("presents the full-window navigation in product priority order", () => {
    render(
      <TraySidebar activeView="home" localOnlyMode onSelectView={vi.fn()} />,
    );

    const navigation = screen.getByRole("navigation", {
      name: "AI Switchboard navigation",
    });
    const labels = Array.from(navigation.querySelectorAll("strong"), (node) =>
      node.textContent?.trim(),
    );

    expect(labels.slice(0, 6)).toEqual([
      "Overview",
      "Optimize",
      "Repo Intelligence",
      "Routing",
      "Workbench",
      "Savings",
    ]);
  });

  it("exposes every visible destination once and marks only the active route", async () => {
    const user = userEvent.setup();
    const onSelectView = vi.fn();
    render(<TraySidebar activeView="repoIntelligence" localOnlyMode={false} onSelectView={onSelectView} />);
    const destinations = [
      ["Overview", "home"], ["Optimize", "optimization"], ["Repo Intelligence", "repoIntelligence"],
      ["Routing", "routingModels"], ["Workbench", "workbench"], ["Savings", "usage"], ["Doctor", "doctor"], ["Token X-Ray", "xray"],
      ["Daily Briefing", "briefing"], ["Agent Memory", "agentMemory"], ["Event Log", "notifications"],
      ["Repo Map", "repoMap"], ["Addons", "addons"], ["Upgrade", "upgrade"], ["Settings", "settings"],
    ] as const;
    for (const [label, route] of destinations) {
      const button = screen.getByRole("button", { name: label });
      expect(button).toHaveAttribute("title", label);
      await user.click(button);
      expect(onSelectView).toHaveBeenLastCalledWith(route);
    }
    expect(screen.getByRole("button", { name: "Repo Intelligence" })).toHaveAttribute("aria-current", "page");
    expect(screen.getAllByRole("button").filter((button) => button.getAttribute("aria-current") === "page")).toHaveLength(1);
  });

  it("routes main nav and footer actions through native button activation", async () => {
    const user = userEvent.setup();
    const onSelectView = vi.fn();
    render(
      <TraySidebar
        activeView="home"
        localOnlyMode={false}
        onSelectView={onSelectView}
      />,
    );

    await user.click(screen.getByRole("button", { name: /repo map/i }));
    expect(onSelectView).toHaveBeenLastCalledWith("repoMap");

    screen.getByRole("button", { name: /settings/i }).focus();
    await user.keyboard("{Enter}");
    expect(onSelectView).toHaveBeenLastCalledWith("settings");

    screen.getByRole("button", { name: /upgrade/i }).focus();
    await user.keyboard(" ");
    expect(onSelectView).toHaveBeenLastCalledWith("upgrade");
  });

  it("hides upgrade entry in local-only mode", () => {
    render(
      <TraySidebar activeView="home" localOnlyMode onSelectView={vi.fn()} />,
    );

    expect(
      screen.queryByRole("button", { name: /upgrade/i }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /settings/i })).toBeInTheDocument();
  });
});
