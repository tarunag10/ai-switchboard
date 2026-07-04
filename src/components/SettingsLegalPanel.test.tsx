import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { SettingsLegalPanel } from "./SettingsLegalPanel";

describe("SettingsLegalPanel", () => {
  it("renders bundled Terms and Privacy offline without links", () => {
    render(<SettingsLegalPanel requiredTermsVersion={2} />);

    expect(
      screen.getByRole("heading", { name: "AI Switchboard Terms of Use" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", {
        name: "AI Switchboard Privacy Notice",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", {
        name: "Licenses, Notices, and Assets",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Terms version 2/i)).toBeInTheDocument();
    expect(
      screen.getByText(/remote account, billing, checkout, or paid pricing/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Review exported diagnostics before sharing them/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/source code is MIT-licensed/i)).toBeInTheDocument();
    expect(screen.getByText(/bundled NOTICE explains/i)).toBeInTheDocument();
    expect(screen.getByText(/bundled TRADEMARKS notice/i)).toBeInTheDocument();
    expect(screen.getByText(/docs\/asset-provenance\.md/i)).toBeInTheDocument();
    expect(screen.getByText(/npm run check:branding/i)).toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
  });
});
