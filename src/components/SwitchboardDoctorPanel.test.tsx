import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DoctorReport } from "../lib/types";
import { SwitchboardDoctorPanel } from "./SwitchboardDoctorPanel";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const warningReport: DoctorReport = {
  status: "warning",
  summary: "Doctor found switchboard items that may need attention.",
  issues: [
    {
      id: "headroom_runtime_unreachable",
      title: "Headroom runtime is not reachable",
      body: "Repair will restart the Headroom runtime.",
      severity: "error",
      repairAction: "repair_runtime",
    },
    {
      id: "codex_direct_bypass",
      title: "Codex is bypassing Headroom",
      body: "Compact the conversation context, then reset this bypass.",
      severity: "warning",
      repairAction: "reset_codex_bypass",
    },
    {
      id: "codex_provider_mismatch",
      title: "Codex routing config needs repair",
      body: "Repair will re-apply the reversible Codex setup.",
      severity: "warning",
      repairAction: "repair_codex_setup",
    },
    {
      id: "no_headroom_clients",
      title: "No clients are routed through Headroom",
      body: "Repair will re-apply reversible client setup.",
      severity: "warning",
      repairAction: "repair_client_setups",
    },
    {
      id: "rtk_not_active",
      title: "RTK is not active",
      body: "Repair will install RTK and enable local shell compression.",
      severity: "warning",
      repairAction: "repair_rtk_runtime",
    },
    {
      id: "rtk_integration_incomplete",
      title: "RTK integration is incomplete",
      body: "Repair will re-apply the local RTK integration.",
      severity: "warning",
      repairAction: "repair_rtk_integrations",
    },
  ],
};

describe("SwitchboardDoctorPanel", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ codexThreadRetagging: "ask" });
  });

  it("shows when the report is healthy", () => {
    render(
      <SwitchboardDoctorPanel
        report={{ status: "ok", summary: "No issues.", issues: [] }}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    expect(screen.getByLabelText("Switchboard Doctor")).toHaveClass(
      "switchboard-doctor--ok",
    );
    expect(screen.getByText("Ready")).toBeInTheDocument();
    expect(screen.getByText("No issues.")).toBeInTheDocument();
    expect(
      screen.getByText("No manual follow-up is needed right now."),
    ).toBeInTheDocument();
    expect(screen.queryByText(/manual-only warnings/i)).not.toBeInTheDocument();
  });

  it("shows successful repair message when report is healthy", () => {
    render(
      <SwitchboardDoctorPanel
        report={{ status: "ok", summary: "No issues.", issues: [] }}
        busyAction={null}
        error={null}
        successMessage="Repair complete. Switchboard looks ready."
        onRepair={vi.fn()}
      />,
    );

    expect(screen.getByRole("heading", { name: "Ready" })).toBeInTheDocument();
    expect(screen.getByLabelText("Switchboard Doctor")).toHaveClass(
      "switchboard-doctor--ok",
    );
    expect(
      screen.getByText("Repair complete. Switchboard looks ready."),
    ).toBeInTheDocument();
  });

  it("renders issues and runs repair actions", async () => {
    const user = userEvent.setup();
    const onRepair = vi.fn();
    render(
      <SwitchboardDoctorPanel
        report={warningReport}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={onRepair}
      />,
    );

    expect(
      screen.getByRole("heading", { name: "Needs attention" }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Switchboard Doctor")).toHaveClass(
      "switchboard-doctor--warning",
    );
    expect(screen.getByText("6 automatic")).toBeInTheDocument();
    expect(screen.getByText("0 manual")).toBeInTheDocument();
    expect(
      screen.queryByText("Repair all will leave manual steps visible."),
    ).not.toBeInTheDocument();
    expect(screen.getByText("Codex is bypassing Headroom")).toBeInTheDocument();
    expect(
      screen.getByText("Codex routing config needs repair"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Compact the Codex conversation, switch to RTK only for parallel heavy goals, then reset the Codex bypass when you want Headroom routing again.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Repair Codex setup to re-apply the managed provider block, then choose a Codex-supported ChatGPT model before retrying.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Installs or enables RTK in managed storage for local shell-output compression.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Restores RTK PATH and hook wiring without reinstalling the binary.",
      ),
    ).toBeInTheDocument();

    expect(
      screen.getByRole("button", { name: "Repair all" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Restart Headroom" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Repair all managed clients" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Repair Codex" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Install RTK" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Repair RTK" }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Reset Codex" }));
    expect(onRepair).toHaveBeenCalledWith("reset_codex_bypass");
    await user.click(screen.getByRole("button", { name: "Repair Codex" }));
    expect(onRepair).toHaveBeenCalledWith("repair_codex_setup");
    await user.click(screen.getByRole("button", { name: "Install RTK" }));
    expect(onRepair).toHaveBeenCalledWith("repair_rtk_runtime");
    await user.click(screen.getByRole("button", { name: "Repair all" }));
    expect(onRepair).toHaveBeenCalledWith("repair_all");
  });

  it("loads and updates Codex retagging consent", async () => {
    const user = userEvent.setup();
    invokeMock
      .mockResolvedValueOnce({ codexThreadRetagging: "disabled" })
      .mockResolvedValueOnce({ codexThreadRetagging: "enabled" });

    render(
      <SwitchboardDoctorPanel
        report={{ status: "ok", summary: "No issues.", issues: [] }}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    expect(await screen.findByText("disabled")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "enabled" }));

    expect(invokeMock).toHaveBeenLastCalledWith(
      "set_codex_thread_retagging_settings",
      { settings: { codexThreadRetagging: "enabled" } },
    );
    expect(screen.getByText("Codex retagging set to enabled.")).toBeInTheDocument();
  });

  it("shows busy and error states", () => {
    render(
      <SwitchboardDoctorPanel
        report={warningReport}
        busyAction="repair_all"
        error="Could not repair."
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("button", { name: "Repairing all" }),
    ).toBeDisabled();
    expect(screen.getByText("Could not repair.")).toBeInTheDocument();
  });

  it("copies a shareable Doctor report", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    render(
      <SwitchboardDoctorPanel
        report={warningReport}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Doctor report" }));

    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0][0]).toContain(
      "Mac AI Switchboard Doctor report",
    );
    expect(writeText.mock.calls[0][0]).toContain(
      "Action: automatic / Reset Codex",
    );
    expect(
      screen.queryByRole("button", { name: "Verify Off" }),
    ).not.toBeInTheDocument();
    expect(screen.getByText("Copied report.")).toBeInTheDocument();
  });

  it("copies a shareable Doctor timeline", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    render(
      <SwitchboardDoctorPanel
        report={warningReport}
        busyAction={null}
        error={null}
        successMessage="Repair complete. Switchboard looks ready."
        onRepair={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Timeline" }));

    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0][0]).toContain(
      "Mac AI Switchboard Doctor timeline",
    );
    expect(writeText.mock.calls[0][0]).toContain("Doctor status: warning");
    expect(writeText.mock.calls[0][0]).toContain("Latest repair completed");
    expect(writeText.mock.calls[0][0]).toContain(
      "Repo Intelligence Doctor availability gates",
    );
    expect(writeText.mock.calls[0][0]).toContain(
      "get_index_freshness is the trust gate",
    );
    expect(writeText.mock.calls[0][0]).toContain(
      "clear_repo_index removes only Switchboard managed index metadata",
    );
    expect(screen.getByText("Copied timeline.")).toBeInTheDocument();
  });

  it("copies failed repair details into the Doctor timeline", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    render(
      <SwitchboardDoctorPanel
        report={warningReport}
        busyAction={null}
        error="gemini_cli repair applied but verification still failed: GEMINI_API_KEY=sk-proj-secret missing from /Users/tarunagarwal/.zshrc."
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Timeline" }));

    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0][0]).toContain("Latest repair failed");
    expect(writeText.mock.calls[0][0]).toContain("Kind: Failed repair");
    expect(writeText.mock.calls[0][0]).toContain("GEMINI_API_KEY=[secret]");
    expect(writeText.mock.calls[0][0]).toContain("[user-path]");
  });

  it("copies the Rollback Center inventory from Doctor", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    render(
      <SwitchboardDoctorPanel
        report={{ status: "ok", summary: "No issues.", issues: [] }}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    await user.click(
      screen.getByRole("button", { name: "Rollback inventory" }),
    );

    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0][0]).toContain(
      "Mac AI Switchboard Rollback Center inventory",
    );
    expect(writeText.mock.calls[0][0]).toContain(
      "No files are changed by this report.",
    );
    expect(writeText.mock.calls[0][0]).toContain("## Codex routing");
    expect(writeText.mock.calls[0][0]).toContain("Marker: headroom:codex_cli");
    expect(writeText.mock.calls[0][0]).toContain(
      "Remove managed Codex shell routing",
    );
    expect(writeText.mock.calls[0][0]).toContain(
      "## Amazon Q Developer CLI routing",
    );
    expect(writeText.mock.calls[0][0]).toContain(
      "AWS credentials, SSO cache, and profiles are not modified",
    );
    expect(screen.getByText("Copied Rollback Center.")).toBeInTheDocument();
  });

  it("offers Repo Memory MCP prepare as an automatic Doctor repair", async () => {
    const user = userEvent.setup();
    const onRepair = vi.fn();

    render(
      <SwitchboardDoctorPanel
        report={{
          status: "warning",
          summary: "Doctor found switchboard items that may need attention.",
          issues: [
            {
              id: "repo_memory_mcp_not_configured",
              title: "Repo Memory MCP is not configured",
              body: "repo-memory missing from Claude MCP config",
              severity: "warning",
              repairAction: "install_repo_memory_mcp",
            },
          ],
        }}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={onRepair}
      />,
    );

    expect(screen.getByText("1 automatic")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Use Prepare MCP in Mode Inspector for the one-click install, start, and smoke check before asking supported agents to consume repo-memory tools.",
      ),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Prepare MCP" }));

    expect(onRepair).toHaveBeenCalledWith("install_repo_memory_mcp");
  });

  it("copies a focused Verify Off report when Off mode evidence remains", async () => {
    const user = userEvent.setup();
    const onRepair = vi.fn();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    render(
      <SwitchboardDoctorPanel
        report={{
          status: "warning",
          summary: "Off mode requested, but routing is still visible.",
          issues: [
            {
              id: "off_mode_not_clean",
              title: "Off mode still has active routing evidence",
              body: "Headroom engine is still reachable.",
              severity: "warning",
              repairAction: "verify_off_mode",
            },
          ],
        }}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={onRepair}
      />,
    );

    expect(screen.getByText("0 automatic")).toBeInTheDocument();
    expect(screen.getByText("0 manual")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Verification" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Verification", { selector: "span" })).toHaveClass(
      "switchboard-doctor__action-kind--verification",
    );
    expect(
      screen.queryByRole("heading", { name: "Manual review" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Repair all" }),
    ).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Verify Off" }));
    expect(onRepair).toHaveBeenCalledWith("verify_off_mode");

    await user.click(screen.getByTitle("Copy Verify Off report"));

    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0][0]).toContain(
      "Mac AI Switchboard Verify Off report",
    );
    expect(writeText.mock.calls[0][0]).toContain(
      "Status: active routing evidence found",
    );
  });

  it("copies gated connector readiness dossiers when connector evidence is present", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    render(
      <SwitchboardDoctorPanel
        report={{
          status: "warning",
          summary: "Gated connector readiness detected.",
          issues: [
            {
              id: "planned_connectors_detected",
              title: "Gated connector readiness detected",
              body: "Gemini CLI detected.",
              severity: "warning",
              repairAction: null,
            },
          ],
        }}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    await user.click(
      screen.getByRole("button", { name: "Connector dossiers" }),
    );

    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0][0]).toContain(
      "Connector config readiness dossiers",
    );
    expect(writeText.mock.calls[0][0]).toContain(
      "No ungated connector-native write dossiers remain",
    );
    expect(writeText.mock.calls[0][0]).toContain("managed connector coverage");
    expect(writeText.mock.calls[0][0]).toContain("promoted routing evidence");
    expect(screen.getByText("Copied connector dossiers.")).toBeInTheDocument();
  });
});
