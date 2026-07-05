import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { SettingsImportPreview } from "../lib/settingsTransfer";
import { SettingsTransferCard } from "./SettingsTransferCard";

const preview: SettingsImportPreview = {
  valid: true,
  title: "Ready to import",
  detail: "2 safe preferences can be applied.",
  errors: [],
  safePreferences: {
    switchboardMode: "full",
    savingsMode: "balanced",
  },
  migrationActions: Array.from({ length: 10 }, (_, index) => ({
    id: `action-${index}`,
    label: `Action ${index}`,
    status: "manual" as const,
    detail: `Detail ${index}`,
  })),
  manualItems: Array.from({ length: 8 }, (_, index) => `Manual ${index}`),
};

describe("SettingsTransferCard", () => {
  it("renders safe settings preview and caps review lists", () => {
    render(
      <SettingsTransferCard
        switchboardMode="full"
        savingsMode="balanced"
        connectorCount={4}
        addonCount={3}
        importText="{}"
        importPreview={preview}
        importBusy={false}
        notice="Export copied."
        onCopyExport={vi.fn()}
        onImportTextChange={vi.fn()}
        onPreviewImport={vi.fn()}
        onApplyImport={vi.fn()}
      />,
    );

    expect(screen.getByText("Settings import/export")).toBeInTheDocument();
    expect(screen.getByText("Export copied.")).toBeInTheDocument();
    expect(screen.getByText("Ready to import")).toBeInTheDocument();
    expect(screen.getByText("Action 7")).toBeInTheDocument();
    expect(screen.queryByText("Action 8")).not.toBeInTheDocument();
    expect(screen.getByText("Manual 5")).toBeInTheDocument();
    expect(screen.queryByText("Manual 6")).not.toBeInTheDocument();
  });

  it("reports text changes and blocks apply until preview is valid", async () => {
    const user = userEvent.setup();
    const onImportTextChange = vi.fn();
    const onApplyImport = vi.fn();

    render(
      <SettingsTransferCard
        switchboardMode="full"
        savingsMode="balanced"
        connectorCount={0}
        addonCount={0}
        importText=""
        importPreview={null}
        importBusy={false}
        notice={null}
        onCopyExport={vi.fn()}
        onImportTextChange={onImportTextChange}
        onPreviewImport={vi.fn()}
        onApplyImport={onApplyImport}
      />,
    );

    expect(screen.getByRole("button", { name: "Apply safe preferences" })).toBeDisabled();
    await user.type(screen.getByRole("textbox"), "safe preferences");

    expect(onImportTextChange).toHaveBeenCalled();
    expect(onApplyImport).not.toHaveBeenCalled();
  });
});
