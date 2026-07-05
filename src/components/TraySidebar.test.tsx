import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { TraySidebar } from "./TraySidebar";

describe("TraySidebar", () => {
  it("routes main nav and footer actions through native button activation", async () => {
    const user = userEvent.setup();
    const onSelectView = vi.fn();

    render(
      <TraySidebar activeView="home" localOnlyMode={false} onSelectView={onSelectView} />,
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
    render(<TraySidebar activeView="home" localOnlyMode onSelectView={vi.fn()} />);

    expect(screen.queryByRole("button", { name: /upgrade/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /settings/i })).toBeInTheDocument();
  });
});

