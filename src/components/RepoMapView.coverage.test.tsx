import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import repoMapJson from "../../docs/repo-map/repo-map.json";
import { RepoMapView } from "./RepoMapView";
import type {
  RepoMapGenerationResponse,
  RepoMapPreflightResponse,
  RepoMapSnapshot,
} from "../lib/repoMapJob";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  open: vi.fn(),
  listen: vi.fn(),
  eventHandler: null as null | ((event: { payload: unknown }) => void),
  unlisten: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.open }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, handler: (event: { payload: unknown }) => void) => {
    mocks.listen(name, handler);
    mocks.eventHandler = handler;
    return Promise.resolve(mocks.unlisten);
  },
}));

const readyPreflight: RepoMapPreflightResponse = {
  repoPath: "/work/repo",
  exists: true,
  isDirectory: true,
  hasPackageJson: true,
  hasCargoManifest: true,
  tools: [
    { id: "graphify", label: "Graphify", available: true, detail: "ready", installHint: null },
  ],
};

const generated: RepoMapGenerationResponse = {
  repoPath: "/work/repo",
  outDir: "/work/repo/docs/repo-map",
  readmePath: "/work/repo/docs/repo-map/README.md",
  compactContextPath: "/work/repo/docs/repo-map/compact-context.md",
  map: repoMapJson as unknown as RepoMapSnapshot,
  compactContext: "bounded generated context",
  toolLog: {},
  stdoutTail: "generation stdout",
  stderrTail: "generation warning",
};

function invokeByCommand(command: string) {
  if (command === "preflight_repo_map") return Promise.resolve(readyPreflight);
  if (command === "generate_repo_map") return Promise.resolve(generated);
  if (command === "cancel_repo_map_generation") return Promise.resolve(false);
  if (command === "open_repo_map_artifact") return Promise.resolve(true);
  return Promise.reject(new Error(`Unexpected command: ${command}`));
}

function renderView() {
  const onOpenDoctor = vi.fn();
  const onOpenRepoIntelligence = vi.fn();
  return {
    ...render(<RepoMapView onOpenDoctor={onOpenDoctor} onOpenRepoIntelligence={onOpenRepoIntelligence} />),
    onOpenDoctor,
    onOpenRepoIntelligence,
  };
}

describe("RepoMapView job and artifact flows", () => {
  beforeEach(() => {
    localStorage.clear();
    mocks.invoke.mockReset();
    mocks.invoke.mockImplementation(invokeByCommand);
    mocks.open.mockReset();
    mocks.open.mockResolvedValue(null);
    mocks.listen.mockClear();
    mocks.unlisten.mockClear();
    mocks.eventHandler = null;
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn(() => Promise.resolve()) },
    });
    delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it("preflights the default path, then checks the edited path exactly", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    renderView();
    await screen.findByText("Path ready");
    expect(mocks.invoke).toHaveBeenCalledWith("preflight_repo_map", {
      repoPath: "/Users/tarunagarwal/Developer/Codex-Repos/mac-ai-switchboard",
    });

    const input = screen.getByRole("textbox", { name: "Repository path" });
    await user.clear(input);
    await user.type(input, " /work/edited ");
    await user.click(screen.getByRole("button", { name: "Check" }));
    expect(mocks.invoke).toHaveBeenLastCalledWith("preflight_repo_map", {
      repoPath: "/work/edited",
    });
    expect(await screen.findByText("Preflight complete.")).toBeVisible();
  });

  it("uses the native folder picker and preflights its selected directory", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    mocks.open.mockResolvedValue("/picked/repo");
    renderView();
    await screen.findByText("Path ready");
    await user.click(screen.getByRole("button", { name: "Browse" }));

    expect(mocks.open).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
      title: "Choose repository folder",
    });
    expect(mocks.invoke).toHaveBeenLastCalledWith("preflight_repo_map", {
      repoPath: "/picked/repo",
    });
    expect(screen.getByDisplayValue("/picked/repo")).toBeVisible();
    expect(screen.getByText("Repository folder selected.")).toBeVisible();
  });

  it("generates, persists history, copies output, opens artifacts, and reloads history paths", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    const clipboard = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: clipboard } });
    renderView();
    await screen.findByText("Path ready");

    const input = screen.getByRole("textbox", { name: "Repository path" });
    await user.clear(input);
    await user.type(input, "/work/repo");
    await user.click(screen.getByRole("button", { name: "Generate repo map" }));
    expect(mocks.invoke).toHaveBeenCalledWith("generate_repo_map", { repoPath: "/work/repo" });
    expect(await screen.findByText("Map ready")).toBeVisible();
    expect(localStorage.getItem("mac-ai-switchboard:repoMapHistory")).toContain("/work/repo");

    await user.click(screen.getByRole("button", { name: "Copy compact context" }));
    expect(clipboard).toHaveBeenCalledWith("bounded generated context");
    expect(await screen.findByText("Compact context copied.")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "README" }));
    expect(mocks.invoke).toHaveBeenCalledWith("open_repo_map_artifact", {
      request: { repoPath: "/work/repo", artifact: "readme" },
    });
    expect(await screen.findByText("Opened artifact.")).toBeVisible();

    await user.click(screen.getByRole("button", { name: /\/work\/repo/ }));
    expect(screen.getByText("History path loaded. Generate to refresh this map.")).toBeVisible();
  });

  it("shows missing-tool remediation and copies its exact install hint", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    const missing: RepoMapPreflightResponse = {
      ...readyPreflight,
      tools: [{ id: "graphify", label: "Graphify", available: false, detail: "missing", installHint: "uv tool install graphify" }],
    };
    mocks.invoke.mockResolvedValueOnce(missing);
    const clipboard = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: clipboard } });
    renderView();
    const details = await screen.findByRole("button", { name: "Details" });
    expect(details).toHaveAttribute("aria-expanded", "false");
    await user.click(details);
    expect(screen.getByLabelText("Repo map tool install checks")).toHaveTextContent("uv tool install graphify");
    await user.click(screen.getByRole("button", { name: "Learn more" }));
    await user.click(screen.getByRole("button", { name: "Copy fix" }));
    expect(clipboard).toHaveBeenCalledWith("uv tool install graphify");
    expect(await screen.findByText("Install hint copied.")).toBeVisible();
  });

  it("surfaces generation failure and retries successfully", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    mocks.invoke
      .mockResolvedValueOnce(readyPreflight)
      .mockRejectedValueOnce(new Error("graph tool crashed"))
      .mockResolvedValueOnce(generated)
      .mockResolvedValue(readyPreflight);
    renderView();
    await screen.findByText("Path ready");
    await user.click(screen.getByRole("button", { name: "Generate repo map" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("graph tool crashed");
    await user.click(screen.getByRole("button", { name: "Retry generation" }));
    expect(await screen.findByText("Map ready")).toBeVisible();
    expect(mocks.invoke).toHaveBeenCalledTimes(4);
  });

  it("offers cancellation during a live run and reports an already-finished job", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    let resolveGeneration!: (value: RepoMapGenerationResponse) => void;
    const pending = new Promise<RepoMapGenerationResponse>((resolve) => { resolveGeneration = resolve; });
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "preflight_repo_map") return Promise.resolve(readyPreflight);
      if (command === "generate_repo_map") return pending;
      if (command === "cancel_repo_map_generation") return Promise.resolve(false);
      return Promise.resolve(true);
    });
    renderView();
    await screen.findByText("Path ready");
    await user.click(screen.getByRole("button", { name: "Generate repo map" }));
    expect(screen.getByRole("button", { name: "Cancel" })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(mocks.invoke).toHaveBeenCalledWith("cancel_repo_map_generation");
    expect(await screen.findByText("No Repo Map generation is currently running.")).toBeVisible();
    await act(async () => resolveGeneration(generated));
  });

  it("renders matching desktop progress events and ignores events from other repositories", async () => {
    (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" } },
      transformCallback: () => {},
    };
    renderView();
    await waitFor(() => expect(mocks.listen).toHaveBeenCalledWith("repo_map_generation_event", expect.any(Function)));

    act(() => mocks.eventHandler?.({ payload: {
      repoPath: "/other/repo", phase: "running", stream: "stdout", message: "ignore me",
    } }));
    expect(screen.queryByText("ignore me")).not.toBeInTheDocument();

    act(() => mocks.eventHandler?.({ payload: {
      repoPath: "/Users/tarunagarwal/Developer/Codex-Repos/mac-ai-switchboard",
      phase: "running", stream: "status", message: "Scanning imports", toolId: "madge",
      toolStatus: "ok", progressPercent: 40, completedTools: 2, totalTools: 5,
    } }));
    expect(await screen.findByText("Scanning imports")).toBeVisible();
    expect(screen.getByRole("button", { name: "Hide run output" })).toHaveAttribute("aria-expanded", "true");
  });
});
