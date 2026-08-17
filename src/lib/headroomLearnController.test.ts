import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { CLAUDE_CODE_INSTALL_DOCS_URL } from "./cliInstallCommands";
import { useHeadroomLearnController } from "./headroomLearnController";
import type {
  ClaudeCodeProject,
  ClientConnectorStatus,
  RuntimeStatus,
} from "./types";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const projects = [
  {
    projectPath: "/repo/older",
    displayName: "Older",
    lastWorkedAt: "2026-01-01T00:00:00.000Z",
  },
  {
    projectPath: "/repo/newer",
    displayName: "Newer",
    lastWorkedAt: "2026-02-01T00:00:00.000Z",
  },
] as ClaudeCodeProject[];

function baseProps() {
  return {
    activeView: "home" as const,
    trayWindowFocused: false,
    runtimeStatus: null as RuntimeStatus | null,
    connectors: [] as ClientConnectorStatus[],
    claudeProjects: projects,
    setClaudeProjects: vi.fn(),
    refreshClaudeProjects: vi.fn(async () => undefined),
    openExternalLink: vi.fn(async () => undefined),
  };
}

describe("useHeadroomLearnController", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.useRealTimers();
  });

  it("sorts projects newest first and limits the initial visible list", () => {
    const props = baseProps();
    const { result } = renderHook(() => useHeadroomLearnController(props));
    expect(result.current.sortedClaudeProjects.map((item) => item.displayName)).toEqual([
      "Newer",
      "Older",
    ]);
    expect(result.current.visibleClaudeProjects).toHaveLength(2);
    expect(result.current.headroomLearnSupported).toBe(true);
  });

  it("describes Codex-only and combined learning sources", () => {
    const codex = {
      clientId: "codex",
      name: "Codex",
      enabled: true,
    } as ClientConnectorStatus;
    const claude = {
      clientId: "claude_code",
      name: "Claude Code",
      enabled: true,
    } as ClientConnectorStatus;
    const codexOnly = renderHook(() =>
      useHeadroomLearnController({ ...baseProps(), connectors: [codex] }),
    );
    expect(codexOnly.result.current.codexLearnEnabled).toBe(true);
    expect(codexOnly.result.current.learnBlurb).toContain("Codex sessions");

    const combined = renderHook(() =>
      useHeadroomLearnController({
        ...baseProps(),
        connectors: [codex, claude],
      }),
    );
    expect(combined.result.current.claudeLearnEnabled).toBe(true);
    expect(combined.result.current.learnBlurb).toContain(
      "Claude Code and Codex sessions",
    );
  });

  it("shows all projects on request", () => {
    const manyProjects = [
      ...projects,
      ...[3, 4, 5].map(
        (index) =>
          ({
            projectPath: `/repo/${index}`,
            displayName: `Project ${index}`,
            lastWorkedAt: `2026-03-0${index}T00:00:00.000Z`,
          }) as ClaudeCodeProject,
      ),
    ];
    const { result } = renderHook(() =>
      useHeadroomLearnController({
        ...baseProps(),
        claudeProjects: manyProjects,
      }),
    );
    expect(result.current.visibleClaudeProjects).toHaveLength(3);
    act(() => result.current.setShowAllClaudeProjects(true));
    expect(result.current.visibleClaudeProjects).toHaveLength(5);
  });

  it("refreshes prerequisites with the exact force payload and falls back safely", async () => {
    invokeMock.mockResolvedValueOnce({
      claudeCliAvailable: true,
      claudeCliPath: "/bin/claude",
      codexCliAvailable: true,
      codexCliPath: "/bin/codex",
      codexLoggedIn: true,
    });
    const { result } = renderHook(() => useHeadroomLearnController(baseProps()));
    await act(() => result.current.refreshHeadroomLearnPrereq(true));
    expect(invokeMock).toHaveBeenCalledWith(
      "get_headroom_learn_prereq_status",
      { force: true },
    );
    expect(result.current.headroomLearnPrereq.codexLoggedIn).toBe(true);

    invokeMock.mockRejectedValueOnce(new Error("missing"));
    await act(() => result.current.refreshHeadroomLearnPrereq());
    expect(result.current.headroomLearnPrereq.claudeCliAvailable).toBe(false);
  });

  it("does not start when the requested agent prerequisite is unavailable", async () => {
    invokeMock.mockResolvedValueOnce({
      claudeCliAvailable: false,
      claudeCliPath: null,
      codexCliAvailable: false,
      codexCliPath: null,
      codexLoggedIn: false,
    });
    const { result } = renderHook(() => useHeadroomLearnController(baseProps()));
    await act(() => result.current.handleRunHeadroomLearn("claude", "/repo/newer"));
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).not.toHaveBeenCalledWith(
      "start_headroom_learn",
      expect.anything(),
    );
  });

  it("stops before learn when prerequisite discovery fails", async () => {
    invokeMock.mockRejectedValueOnce(new Error("probe failed"));
    const { result } = renderHook(() => useHeadroomLearnController(baseProps()));
    await act(() => result.current.handleRunHeadroomLearn("codex"));
    expect(result.current.headroomLearnPrereq.codexCliAvailable).toBe(false);
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("loads optimization prerequisites and applied patterns", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_headroom_learn_prereq_status") {
        return {
          claudeCliAvailable: true,
          codexCliAvailable: true,
          codexLoggedIn: true,
        };
      }
      if (command === "list_applied_patterns_for_projects") {
        return { "/repo/newer": { count: 2 } };
      }
      throw new Error(`unexpected ${command}`);
    });
    const props = { ...baseProps(), activeView: "optimization" as const };
    const { result } = renderHook(() => useHeadroomLearnController(props));
    await waitFor(() =>
      expect(result.current.optimizeAppliedByProject).toEqual({
        "/repo/newer": { count: 2 },
      }),
    );
    expect(props.refreshClaudeProjects).toHaveBeenCalledOnce();
    expect(invokeMock).toHaveBeenCalledWith(
      "list_applied_patterns_for_projects",
      { projectPaths: ["/repo/newer", "/repo/older"] },
    );
  });

  it("falls back when applied-pattern loading fails and handles empty projects", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_headroom_learn_prereq_status") throw new Error("no cli");
      if (command === "list_applied_patterns_for_projects") {
        throw new Error("no patterns");
      }
      return undefined;
    });
    const { result, rerender } = renderHook(
      ({ claudeProjects }) =>
        useHeadroomLearnController({
          ...baseProps(),
          activeView: "optimization",
          claudeProjects,
        }),
      { initialProps: { claudeProjects: projects } },
    );
    await waitFor(() => expect(result.current.optimizeAppliedByProject).toBeNull());
    rerender({ claudeProjects: [] });
    await waitFor(() =>
      expect(result.current.optimizeAppliedByProject).toEqual({}),
    );
  });

  it("polls focused learn status and reports polling failures", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_headroom_learn_prereq_status") return {};
      if (command === "list_applied_patterns_for_projects") return {};
      if (command === "get_headroom_learn_status") {
        return {
          running: true,
          progressPercent: 50,
          summary: "Learning",
          outputTail: [],
        };
      }
      return undefined;
    });
    const props = {
      ...baseProps(),
      activeView: "optimization" as const,
      trayWindowFocused: true,
    };
    const { result, unmount } = renderHook(() =>
      useHeadroomLearnController(props),
    );
    await waitFor(() => expect(result.current.headroomLearnStatus.running).toBe(true));
    unmount();

    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_headroom_learn_status") throw new Error("poll failed");
      if (command === "list_applied_patterns_for_projects") return {};
      return {};
    });
    const failed = renderHook(() => useHeadroomLearnController(props));
    await waitFor(() =>
      expect(failed.result.current.headroomLearnStatus.summary).toBe(
        "Could not read headroom learn status.",
      ),
    );
    failed.unmount();
  });

  it("blocks unsupported platforms before invoking native learn", async () => {
    const props = {
      ...baseProps(),
      runtimeStatus: {
        headroomLearnSupported: false,
        headroomLearnDisabledReason: "Unsupported architecture",
      } as RuntimeStatus,
    };
    invokeMock.mockResolvedValueOnce({
      claudeCliAvailable: true,
      codexCliAvailable: false,
      codexLoggedIn: false,
    });
    const { result } = renderHook(() => useHeadroomLearnController(props));
    await act(() => result.current.handleRunHeadroomLearn("claude", "/repo/newer"));
    expect(result.current.headroomLearnStatus).toMatchObject({
      running: false,
      error: "Unsupported architecture",
    });
    expect(invokeMock).not.toHaveBeenCalledWith(
      "start_headroom_learn",
      expect.anything(),
    );
  });

  it("starts Claude learn with an exact payload and polls to completion", async () => {
    vi.useFakeTimers();
    invokeMock
      .mockResolvedValueOnce({
        claudeCliAvailable: true,
        codexCliAvailable: false,
        codexLoggedIn: false,
      })
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({
        running: false,
        success: true,
        projectPath: "/repo/newer",
        progressPercent: 100,
        summary: "Learn complete.",
        outputTail: [],
      });
    const props = baseProps();
    const { result } = renderHook(() => useHeadroomLearnController(props));
    let run!: Promise<void>;
    act(() => {
      run = result.current.handleRunHeadroomLearn("claude", "/repo/newer");
    });
    await vi.advanceTimersByTimeAsync(180);
    await act(() => run);

    expect(invokeMock).toHaveBeenCalledWith("start_headroom_learn", {
      agent: "claude",
      projectPath: "/repo/newer",
    });
    expect(invokeMock).toHaveBeenCalledWith("get_headroom_learn_status", {
      projectPath: "/repo/newer",
    });
    expect(result.current.headroomLearnStatus.summary).toBe("Learn complete.");
    expect(result.current.headroomLearnBusy).toBe(false);
  });

  it("reports start failures and resets the busy flag", async () => {
    invokeMock
      .mockResolvedValueOnce({
        claudeCliAvailable: false,
        codexCliAvailable: true,
        codexLoggedIn: true,
      })
      .mockRejectedValueOnce(new Error("runner crashed"));
    const { result } = renderHook(() => useHeadroomLearnController(baseProps()));
    await act(() => result.current.handleRunHeadroomLearn("codex"));
    expect(invokeMock).toHaveBeenCalledWith("start_headroom_learn", {
      agent: "codex",
      projectPath: null,
    });
    expect(result.current.headroomLearnStatus).toMatchObject({
      running: false,
      error: "runner crashed",
    });
    expect(result.current.headroomLearnBusy).toBe(false);
  });

  it("copies install commands and preserves a visible fallback on failure", async () => {
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const { result } = renderHook(() => useHeadroomLearnController(baseProps()));
    await act(() => result.current.copyLearnInstallCommand("npm i claude"));
    expect(writeText).toHaveBeenCalledWith("npm i claude");
    expect(result.current.learnInstallCopyNotice).toBe("Copied install command.");

    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: undefined,
    });
    await act(() => result.current.copyLearnInstallCommand("npm i claude"));
    expect(result.current.learnInstallCopyNotice).toBe(
      "Copy failed. Command remains visible below.",
    );
  });

  it("opens official install docs and exposes link failures", async () => {
    const props = baseProps();
    const { result } = renderHook(() => useHeadroomLearnController(props));
    await act(() => result.current.openLearnInstallDocsLink());
    expect(props.openExternalLink).toHaveBeenCalledWith(
      CLAUDE_CODE_INSTALL_DOCS_URL,
    );

    props.openExternalLink.mockRejectedValueOnce(new Error("browser blocked"));
    await act(() => result.current.openLearnInstallDocsLink());
    await waitFor(() =>
      expect(result.current.learnInstallCopyNotice).toBe("browser blocked"),
    );
  });
});
