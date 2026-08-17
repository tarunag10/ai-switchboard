import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SettingsTransferCard, type SettingsTransferCardProps } from "./SettingsTransferCard";

function props(overrides: Partial<SettingsTransferCardProps> = {}): SettingsTransferCardProps {
  return { switchboardMode: "full", savingsMode: "balanced", connectorCount: 2, addonCount: 1, importText: "{}", importPreview: null, importBusy: false, notice: null, onCopyExport: vi.fn(), onImportTextChange: vi.fn(), onPreviewImport: vi.fn(), onApplyImport: vi.fn(), ...overrides };
}

describe("SettingsTransferCard alternate states", () => {
  it("wires export, preview, and valid apply actions", async () => {
    const user = userEvent.setup();
    const p = props({ importPreview: { valid: true, title: "Ready", detail: "Safe", errors: [], safePreferences: { switchboardMode: "off" }, migrationActions: [], manualItems: [] } as never });
    render(<SettingsTransferCard {...p} />);
    await user.click(screen.getByRole("button", { name: "Copy settings export" }));
    await user.click(screen.getByRole("button", { name: "Preview import" }));
    await user.click(screen.getByRole("button", { name: "Apply safe preferences" }));
    expect(p.onCopyExport).toHaveBeenCalledOnce();
    expect(p.onPreviewImport).toHaveBeenCalledOnce();
    expect(p.onApplyImport).toHaveBeenCalledOnce();
    expect(screen.getByText(/Safe preferences: switchboardMode off/)).toBeVisible();
  });

  it("shows invalid errors and keeps apply blocked while busy", () => {
    render(<SettingsTransferCard {...props({ importBusy: true, importPreview: { valid: false, title: "Invalid", detail: "Rejected", errors: ["Unknown schema"], safePreferences: {}, migrationActions: [], manualItems: [] } as never })} />);
    expect(screen.getByText("Unknown schema")).toBeVisible();
    expect(screen.getByRole("button", { name: "Applying..." })).toBeDisabled();
  });
});
