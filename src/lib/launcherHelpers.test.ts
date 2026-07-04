import { describe, expect, it } from "vitest";

import {
  buildInitialProxyVerificationRows,
  getClaudeConnector,
  getContactRequestValidationError,
  getInitialLauncherStage,
  getLauncherAutoConfigureDecision,
  hasPendingOneClickProxyVerification,
  isValidEmailAddress,
  needsTermsAcceptance,
  nextAutoConfigureStep,
  nextAutoConfigureStepAfterApply
} from "./launcherHelpers";
import type { ClientConnectorStatus } from "./types";

describe("launcher helpers", () => {
  it("validates trimmed email addresses for auth and contact flows", () => {
    expect(isValidEmailAddress("  user@example.com  ")).toBe(true);
    expect(isValidEmailAddress("missing-at-symbol")).toBe(false);
    expect(isValidEmailAddress("user@example")).toBe(false);
  });

  it("gates on terms acceptance until the accepted version catches up", () => {
    // New install: nothing accepted yet.
    expect(needsTermsAcceptance(1, 0)).toBe(true);
    // Already accepted the current version.
    expect(needsTermsAcceptance(1, 1)).toBe(false);
    // Terms bumped in an update: previously-accepted version is now stale.
    expect(needsTermsAcceptance(2, 1)).toBe(true);
    // Accepted version somehow ahead (downgrade): do not block.
    expect(needsTermsAcceptance(1, 2)).toBe(false);
  });

  it("returns contact validation errors before submit work begins", () => {
    expect(getContactRequestValidationError(undefined, "user@example.com")).toBe(
      "Set VITE_HEADROOM_CONTACT_FORM_URL to enable contact requests."
    );
    expect(getContactRequestValidationError("https://example.com/form", "invalid")).toBe(
      "Enter a valid email address."
    );
    expect(
      getContactRequestValidationError("https://example.com/form", "user@example.com")
    ).toBeNull();
  });

  it("finds the managed Claude connector from mixed connector lists", () => {
    const connectors: ClientConnectorStatus[] = [
      { clientId: "cursor", name: "Cursor", installed: true, enabled: false, verified: false },
      {
        clientId: "claude_code",
        name: "Claude Code",
        installed: true,
        enabled: true,
        verified: true
      }
    ];

    expect(getClaudeConnector(connectors)).toEqual(connectors[1]);
  });

  it("decides whether launcher auto-setup should wait, apply setup, or continue", () => {
    expect(getLauncherAutoConfigureDecision([])).toBe("show_client_setup");
    expect(
      getLauncherAutoConfigureDecision([
        {
          clientId: "claude_code",
          name: "Claude Code",
          installed: true,
          enabled: false,
          verified: false
        }
      ])
    ).toBe("apply_client_setup");
    expect(
      getLauncherAutoConfigureDecision([
        {
          clientId: "claude_code",
          name: "Claude Code",
          installed: true,
          enabled: true,
          verified: false
        }
      ])
    ).toBe("begin_proxy_verification");
    // Codex-only: a non-Claude tool drives the same auto-configure decision.
    expect(
      getLauncherAutoConfigureDecision([
        { clientId: "codex", name: "Codex", installed: true, enabled: false, verified: false }
      ])
    ).toBe("apply_client_setup");
    expect(
      getLauncherAutoConfigureDecision([
        {
          clientId: "goose",
          name: "Goose",
          setupPhase: "adapt",
          installed: true,
          enabled: false,
          verified: false
        }
      ])
    ).toBe("show_client_setup");
  });

  it("builds initial proxy verification rows from enabled installed Claude connectors", () => {
    const rows = buildInitialProxyVerificationRows([
      { clientId: "cursor", name: "Cursor", supportStatus: "planned", installed: true, enabled: true, verified: false },
      {
        clientId: "claude_code",
        name: "Claude Code",
        installed: true,
        enabled: true,
        verified: false
      },
      {
        clientId: "claude_code",
        name: "Claude Code Beta",
        installed: true,
        enabled: false,
        verified: false
      }
    ]);

    expect(rows).toEqual([
      {
        clientId: "claude_code",
        name: "Claude Code",
        state: "processing",
        message: "Ready to send a Claude Code test prompt.",
        oneClickSupported: true
      }
    ]);
  });

  it("marks one-click verification only for supported pending connector rows", () => {
    expect(
      hasPendingOneClickProxyVerification([
        {
          clientId: "claude_code",
          name: "Claude Code",
          state: "processing",
          message: "Ready",
          oneClickSupported: true
        }
      ])
    ).toBe(true);
    expect(
      hasPendingOneClickProxyVerification([
        {
          clientId: "claude_code",
          name: "Claude Code",
          state: "verified",
          message: "Request received",
          oneClickSupported: true
        },
        {
          clientId: "cursor",
          name: "Cursor",
          state: "waiting",
          message: "Manual",
          oneClickSupported: false
        }
      ])
    ).toBe(false);
  });

  describe("getInitialLauncherStage", () => {
    it("returns null in non-launcher windows regardless of bootstrap state", () => {
      expect(getInitialLauncherStage("dashboard", true, true, "first_run")).toBeNull();
      expect(getInitialLauncherStage("tray", true, true, "resume")).toBeNull();
    });

    it("returns null in the launcher window until bootstrap is complete", () => {
      expect(getInitialLauncherStage("launcher", false, false, "first_run")).toBeNull();
    });

    it("lands first-run users on install when bootstrap completed during startup", () => {
      expect(getInitialLauncherStage("launcher", true, false, "first_run")).toBe("install");
    });

    it("lands first-run users on install when bootstrap was already complete", () => {
      expect(getInitialLauncherStage("launcher", false, true, "first_run")).toBe("install");
    });

    it("lands returning users on post_install once bootstrap is complete", () => {
      expect(getInitialLauncherStage("launcher", true, true, "resume")).toBe("post_install");
      expect(getInitialLauncherStage("launcher", false, true, "dashboard")).toBe(
        "post_install"
      );
    });
  });

  describe("nextAutoConfigureStep", () => {
    const claude: ClientConnectorStatus = {
      clientId: "claude_code",
      name: "Claude Code",
      installed: true,
      enabled: false,
      verified: false
    };

    const codex: ClientConnectorStatus = {
      clientId: "codex",
      name: "Codex",
      installed: true,
      enabled: false,
      verified: false
    };

    it("routes show_client_setup decisions to manual setup", () => {
      expect(nextAutoConfigureStep("show_client_setup", [claude])).toEqual({
        kind: "show_client_setup"
      });
    });

    it("routes apply_client_setup to an apply step for every installed, not-yet-enabled connector", () => {
      expect(nextAutoConfigureStep("apply_client_setup", [claude, codex])).toEqual({
        kind: "apply",
        clientIds: ["claude_code", "codex"]
      });
    });

    it("only applies connectors that are installed and not already enabled", () => {
      expect(
        nextAutoConfigureStep("apply_client_setup", [
          { ...claude, enabled: true },
          codex,
          { clientId: "codex", name: "Codex", installed: false, enabled: false, verified: false }
        ])
      ).toEqual({ kind: "apply", clientIds: ["codex"] });
    });

    it("does not auto-apply detected gated connectors that omit support status", () => {
      expect(
        nextAutoConfigureStep("apply_client_setup", [
          {
            clientId: "goose",
            name: "Goose",
            setupPhase: "adapt",
            installed: true,
            enabled: false,
            verified: false
          }
        ])
      ).toEqual({ kind: "show_client_setup" });
    });

    it("falls back to manual setup when apply_client_setup has no detected connector", () => {
      expect(nextAutoConfigureStep("apply_client_setup", [])).toEqual({
        kind: "show_client_setup"
      });
    });

    it("routes begin_proxy_verification straight to proxy verification", () => {
      expect(nextAutoConfigureStep("begin_proxy_verification", [])).toEqual({
        kind: "begin_proxy_verification"
      });
    });
  });

  describe("nextAutoConfigureStepAfterApply", () => {
    it("advances to proxy verification when apply produced a verified setup", () => {
      expect(nextAutoConfigureStepAfterApply("begin_proxy_verification")).toEqual({
        kind: "begin_proxy_verification"
      });
    });

    it("falls back to manual setup when post-apply state still needs attention", () => {
      expect(nextAutoConfigureStepAfterApply("show_client_setup")).toEqual({
        kind: "show_client_setup"
      });
      expect(nextAutoConfigureStepAfterApply("apply_client_setup")).toEqual({
        kind: "show_client_setup"
      });
    });
  });
});
