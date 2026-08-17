import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { PricingAuthCard } from "./PricingAuthCard";

function renderCard(overrides: Partial<React.ComponentProps<typeof PricingAuthCard>> = {}) {
  const props: React.ComponentProps<typeof PricingAuthCard> = {
    authCode: "",
    authCodeRequestedFor: null,
    authCodeValid: false,
    authEmail: "",
    authEmailValid: false,
    authFlowError: null,
    authFlowSuccess: null,
    authRequestBusy: false,
    authVerifyBusy: false,
    onAuthCodeChange: vi.fn(),
    onAuthEmailChange: vi.fn(),
    onRequestAuthCode: vi.fn(),
    onResetAuthStep: vi.fn(),
    onVerifyAuthCode: vi.fn(),
    pricingError: null,
    upgradeAuthMessage: "Sign in to upgrade",
    ...overrides,
  };
  return { ...render(<PricingAuthCard {...props} />), props };
}

describe("PricingAuthCard", () => {
  it("captures email and enables sign in only after validation", async () => {
    const user = userEvent.setup();
    const { props } = renderCard({ authEmail: "a@example.com", authEmailValid: true });

    await user.type(screen.getByRole("textbox", { name: "Email" }), "x");
    await user.click(screen.getByRole("button", { name: "Sign in" }));

    expect(props.onAuthEmailChange).toHaveBeenLastCalledWith("a@example.comx");
    expect(props.onRequestAuthCode).toHaveBeenCalledOnce();
  });

  it("wires code verification, resend, and email reset", async () => {
    const user = userEvent.setup();
    const { props } = renderCard({
      authCodeRequestedFor: "a@example.com",
      authCode: "12345",
      authCodeValid: true,
    });

    await user.type(screen.getByRole("textbox", { name: "Authentication code" }), "6");
    await user.click(screen.getByRole("button", { name: "Verify and continue" }));
    await user.click(screen.getByRole("button", { name: "Resend code" }));
    await user.click(screen.getByRole("button", { name: "Use a different email" }));

    expect(props.onAuthCodeChange).toHaveBeenLastCalledWith("123456");
    expect(props.onVerifyAuthCode).toHaveBeenCalledOnce();
    expect(props.onRequestAuthCode).toHaveBeenCalledOnce();
    expect(props.onResetAuthStep).toHaveBeenCalledOnce();
  });

  it("surfaces busy states and all independent error/success messages", () => {
    renderCard({
      authCodeRequestedFor: "a@example.com",
      authRequestBusy: true,
      authVerifyBusy: true,
      authFlowError: "Code expired",
      authFlowSuccess: "Signed in",
      pricingError: "Pricing unavailable",
    });

    expect(screen.getByRole("button", { name: "Verifying..." })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Sending..." })).toBeDisabled();
    expect(screen.getByText("Code expired")).toBeVisible();
    expect(screen.getByText("Signed in")).toBeVisible();
    expect(screen.getByText("Pricing unavailable")).toBeVisible();
  });
});
