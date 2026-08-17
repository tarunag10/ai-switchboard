import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { OptimizationView, type OptimizationViewProps } from "./OptimizationView";

vi.mock("./OptimizationDashboard", () => ({ OptimizationDashboard: () => null }));
vi.mock("./OptimizePanel", () => ({ OptimizePanel: ({ onAppliedMutated }: any) => <button onClick={onAppliedMutated}>Applied mock</button> }));

const idleStatus = { running: false, progressPercent: 0, summary: "Idle", outputTail: [], success: null, error: null, projectPath: null } as any;
const project = { id: "project-1", displayName: "Switchboard", projectPath: "/repos/switchboard", sessionCount: 4, modifiedAt: "2026-08-17T00:00:00Z" } as any;

function props(overrides: Partial<OptimizationViewProps> = {}): OptimizationViewProps {
  return {
    activeView: "optimization", setActiveView: vi.fn(), headroomLearnSupported: true, headroomLearnDisabledReason: null,
    headroomLearnPrereq: { claudeCliAvailable: true, codexCliAvailable: true, codexLoggedIn: true }, headroomLearnStatus: idleStatus,
    headroomLearnBusy: false, claudeLearnEnabled: false, codexLearnEnabled: false, claudeProjectsBusy: false,
    claudeProjects: [], visibleClaudeProjects: [], sortedClaudeProjects: [], showAllClaudeProjects: false, setShowAllClaudeProjects: vi.fn(),
    handleRunHeadroomLearn: vi.fn(), copyLearnInstallCommand: vi.fn(), openLearnInstallDocsLink: vi.fn(), refreshHeadroomLearnPrereq: vi.fn(),
    learnInstallCopyNotice: null, optimizeAppliedByProject: null, setOptimizeAppliedRefreshTick: vi.fn(), claudeProjectsError: null,
    learnBlurb: "Learn from local sessions.", ...overrides,
  } as OptimizationViewProps;
}

describe("OptimizationView integrated learning flows", () => {
  it("routes an empty learning setup to Addons and explains unsupported platforms", async () => {
    const user = userEvent.setup();
    const p = props();
    const view = render(<OptimizationView {...p} />);
    await user.click(screen.getByRole("button", { name: "Open Addons" }));
    expect(p.setActiveView).toHaveBeenCalledWith("addons");

    view.rerender(<OptimizationView {...props({ headroomLearnSupported: false, headroomLearnDisabledReason: "Learning unavailable here" })} />);
    expect(screen.getByText("Learning unavailable here")).toBeInTheDocument();
    expect(screen.getByText(/Linux preview currently supports/)).toBeInTheDocument();
  });

  it("wires Claude CLI install, documentation, recheck, scan, and applied refresh actions", async () => {
    const user = userEvent.setup();
    const p = props({
      claudeLearnEnabled: true, claudeProjects: [project], visibleClaudeProjects: [project], sortedClaudeProjects: [project],
      headroomLearnPrereq: { claudeCliAvailable: false, codexCliAvailable: true, codexLoggedIn: true }, learnInstallCopyNotice: "Copied",
    });
    render(<OptimizationView {...p} />);
    expect(screen.getByRole("heading", { name: "Install the Claude Code CLI" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Copy" }));
    await user.click(screen.getByRole("button", { name: "Open install docs" }));
    await user.click(screen.getByRole("button", { name: "Re-check" }));
    expect(p.copyLearnInstallCommand).toHaveBeenCalledWith(expect.stringContaining("curl"));
    expect(p.openLearnInstallDocsLink).toHaveBeenCalledOnce();
    expect(p.refreshHeadroomLearnPrereq).toHaveBeenCalledWith(true);
    expect(screen.getByRole("button", { name: "Scan Claude project" })).toBeDisabled();

    const ready = props({ ...p, headroomLearnPrereq: { claudeCliAvailable: true, codexCliAvailable: true, codexLoggedIn: true } });
    const view = render(<OptimizationView {...ready} />);
    await user.click(screen.getAllByRole("button", { name: "Scan Claude project" })[1]);
    await user.click(screen.getAllByRole("button", { name: "Applied mock" })[1]);
    expect(ready.handleRunHeadroomLearn).toHaveBeenCalledWith("claude", "/repos/switchboard");
    expect(ready.setOptimizeAppliedRefreshTick).toHaveBeenCalledWith(expect.any(Function));
    view.unmount();
  });

  it("selects the Codex install or login command and starts a ready session scan", async () => {
    const user = userEvent.setup();
    const install = props({ codexLearnEnabled: true, headroomLearnPrereq: { claudeCliAvailable: true, codexCliAvailable: false, codexLoggedIn: false } });
    const view = render(<OptimizationView {...install} />);
    expect(screen.getByRole("heading", { name: "Install the Codex CLI" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Copy" }));
    expect(install.copyLearnInstallCommand).toHaveBeenCalledWith(expect.stringContaining("@openai/codex"));

    const login = props({ codexLearnEnabled: true, headroomLearnPrereq: { claudeCliAvailable: true, codexCliAvailable: true, codexLoggedIn: false } });
    view.rerender(<OptimizationView {...login} />);
    expect(screen.getByRole("heading", { name: "Sign in to the Codex CLI" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Copy" }));
    expect(login.copyLearnInstallCommand).toHaveBeenCalledWith(expect.stringContaining("codex login"));

    const ready = props({ codexLearnEnabled: true });
    view.rerender(<OptimizationView {...ready} />);
    await user.click(screen.getByRole("button", { name: "Scan Codex sessions" }));
    expect(ready.handleRunHeadroomLearn).toHaveBeenCalledWith("codex");
  });

  it("renders running and completed failure states and expands long project lists", async () => {
    const user = userEvent.setup();
    const projects = [project, ...[2, 3, 4].map((id) => ({ ...project, id: `project-${id}`, displayName: `Project ${id}`, projectPath: `/repos/${id}` }))];
    const running = props({
      claudeLearnEnabled: true, claudeProjects: projects, visibleClaudeProjects: projects, sortedClaudeProjects: projects,
      headroomLearnStatus: { ...idleStatus, running: true, projectPath: "/repos/switchboard", elapsedSeconds: 7 },
    });
    const view = render(<OptimizationView {...running} />);
    expect(screen.getByText("Scanning sessions · 7s")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Scanning" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "more projects" }));
    expect(running.setShowAllClaudeProjects).toHaveBeenCalledWith(expect.any(Function));

    view.rerender(<OptimizationView {...props({
      claudeLearnEnabled: true, claudeProjects: [project], visibleClaudeProjects: [project], sortedClaudeProjects: [project],
      headroomLearnStatus: { ...idleStatus, projectPath: "/repos/switchboard", success: false, error: "scan failed", outputTail: ["failure"] },
      claudeProjectsError: "project enumeration failed",
    })} />);
    expect(screen.getByText("Last run failed")).toBeInTheDocument();
    expect(screen.getByText("scan failed")).toBeInTheDocument();
    expect(screen.getByText("project enumeration failed")).toBeInTheDocument();
  });
});
