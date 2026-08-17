import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  RepoIntelligencePreview,
  repoIntelligencePreview,
} from "./RepoIntelligencePreview";
import type { RepoIntelligenceSummary } from "../lib/repoIntelligence";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

function realSummary(
  overrides: Partial<RepoIntelligenceSummary> = {},
): RepoIntelligenceSummary {
  return {
    ...repoIntelligencePreview,
    repoRoot: "/work/repo",
    indexedAt: "2026-08-17T00:00:00.000Z",
    ...overrides,
  };
}

const clipboardWrite = vi.fn((_text: string) => Promise.resolve());

function installClipboard(writeText = clipboardWrite) {
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText },
  });
  return writeText;
}

describe("RepoIntelligencePreview backend and copy flows", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
    clipboardWrite.mockReset();
    clipboardWrite.mockResolvedValue(undefined);
    installClipboard();
  });

  it("loads the persisted index and enables every bounded copy format", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    installClipboard();
    const summary = realSummary();
    const onSummaryChange = vi.fn();
    invokeMock.mockResolvedValueOnce(summary);
    render(<RepoIntelligencePreview headroomHealthy rtkHealthy onSummaryChange={onSummaryChange} />);

    expect(await screen.findByDisplayValue("/work/repo")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("get_latest_repo_intelligence_summary");
    expect(onSummaryChange).toHaveBeenCalledWith(summary);
    expect(screen.getAllByText("Fresh local index")).not.toHaveLength(0);

    await user.click(screen.getByRole("button", { name: "Copy pack" }));
    expect(await screen.findByText("Context pack copied.")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Copy agent manifest" }));
    await user.click(screen.getByRole("button", { name: "Copy full handoff" }));
    await user.click(screen.getByRole("button", { name: "Copy summary" }));
    await user.click(screen.getByRole("button", { name: "Copy selected pack" }));
    await user.click(screen.getByRole("button", { name: "Copy JSON" }));
    await user.click(screen.getAllByRole("button", { name: "Copy this pack" })[0]);
    await user.click(screen.getAllByRole("button", { name: "Markdown" })[0]);
    await user.click(screen.getAllByRole("button", { name: "JSON" })[0]);
    await user.click(screen.getAllByRole("button", { name: "Copy agent-ready pack" })[0]);

    expect(clipboardWrite).toHaveBeenCalledTimes(10);
    expect(clipboardWrite.mock.calls.some(([text]) => String(text).includes("repo_agent_handoff"))).toBe(true);
  });

  it("validates a path locally, then indexes with the exact Tauri payload", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    installClipboard();
    const summary = realSummary({ repoRoot: "/trimmed/repo" });
    invokeMock.mockResolvedValueOnce(null).mockResolvedValueOnce(summary);
    const onSummaryChange = vi.fn();
    render(<RepoIntelligencePreview onSummaryChange={onSummaryChange} />);
    await waitFor(() => expect(screen.queryByText("Loading saved index…")).not.toBeInTheDocument());

    await user.click(screen.getByRole("button", { name: "Index" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Enter a local repository folder path first.");
    expect(invokeMock).toHaveBeenCalledTimes(1);

    await user.type(screen.getByRole("textbox", { name: "Repository folder path" }), "  /trimmed/repo  ");
    await user.click(screen.getByRole("button", { name: "Index" }));
    expect(invokeMock).toHaveBeenLastCalledWith("build_repo_intelligence_summary", {
      repoPath: "/trimmed/repo",
    });
    expect(screen.getByRole("textbox", { name: "Repository folder path" })).toHaveValue("  /trimmed/repo  ");
    expect(onSummaryChange).toHaveBeenCalledWith(summary);
  });

  it("reports index failures without discarding the editable path", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    installClipboard();
    invokeMock.mockResolvedValueOnce(null).mockRejectedValueOnce(new Error("permission denied"));
    render(<RepoIntelligencePreview />);
    await waitFor(() => expect(screen.queryByText("Loading saved index…")).not.toBeInTheDocument());
    await user.type(screen.getByRole("textbox", { name: "Repository folder path" }), "/private/repo");
    await user.click(screen.getByRole("button", { name: "Index" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("permission denied");
    expect(screen.getByDisplayValue("/private/repo")).toBeVisible();
  });

  it("recovers from a saved-index load error through the retry action", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    installClipboard();
    const summary = realSummary();
    invokeMock
      .mockRejectedValueOnce(new Error("saved index corrupt"))
      .mockResolvedValueOnce(summary);
    render(<RepoIntelligencePreview />);

    expect(await screen.findByRole("alert")).toHaveTextContent("saved index corrupt");
    await user.click(screen.getByRole("button", { name: "Retry saved index" }));
    expect(await screen.findByDisplayValue("/work/repo")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Retry saved index" })).not.toBeInTheDocument();
  });

  it("clears the saved index and returns copy actions to sample-safe state", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    installClipboard();
    const summary = realSummary();
    const onSummaryChange = vi.fn();
    invokeMock.mockResolvedValueOnce(summary).mockResolvedValueOnce(true);
    render(<RepoIntelligencePreview onSummaryChange={onSummaryChange} />);
    await screen.findByRole("button", { name: "Clear" });
    await user.click(screen.getByRole("button", { name: "Clear" }));

    expect(invokeMock).toHaveBeenLastCalledWith("clear_repo_intelligence_summary");
    await waitFor(() => expect(screen.queryByRole("button", { name: "Clear" })).not.toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Copy full handoff" })).toBeDisabled();
    expect(onSummaryChange).toHaveBeenLastCalledWith(repoIntelligencePreview);
  });

  it("keeps details available and reports clipboard failures", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    installClipboard();
    invokeMock.mockResolvedValueOnce(realSummary());
    clipboardWrite.mockRejectedValueOnce(new Error("clipboard denied"));
    render(<RepoIntelligencePreview />);
    await screen.findByRole("button", { name: "Copy pack" });
    await user.click(screen.getByRole("button", { name: "Copy pack" }));
    expect(await screen.findByText("Copy failed. Pack details remain visible below.")).toBeVisible();
  });

  it("keeps every generated format inspectable when clipboard writes fail", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    installClipboard(vi.fn((_text: string) => Promise.reject(new Error("clipboard denied"))));
    invokeMock.mockResolvedValueOnce(realSummary());
    render(<RepoIntelligencePreview />);
    await screen.findByRole("button", { name: "Copy agent manifest" });

    await user.click(screen.getByRole("button", { name: "Copy agent manifest" }));
    expect(await screen.findByText("Copy failed. Manifest details remain visible below.")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Copy full handoff" }));
    expect(await screen.findByText("Copy failed. Session details remain visible below.")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Copy summary" }));
    expect(await screen.findByText("Copy failed. Session summary remains visible below.")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Copy selected pack" }));
    expect(await screen.findByText("Copy failed. Session pack remains visible below.")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Copy JSON" }));
    expect(await screen.findByText("Copy failed. Session JSON remains visible below.")).toBeVisible();
    await user.click(screen.getAllByRole("button", { name: "Copy this pack" })[0]);
    expect(await screen.findByText("Copy failed. Pack details remain visible below.")).toBeVisible();
    await user.click(screen.getAllByRole("button", { name: "Markdown" })[0]);
    expect(await screen.findByText("Copy failed. Handoff details remain visible below.")).toBeVisible();
    await user.click(screen.getAllByRole("button", { name: "JSON" })[0]);
    expect(await screen.findByText("Copy failed. JSON handoff remains visible below.")).toBeVisible();
    await user.click(screen.getAllByRole("button", { name: "Copy agent-ready pack" })[0]);
    expect(await screen.findByText("Copy failed. Pack details remain visible below.")).toBeVisible();
  });

  it("reports clear failures while preserving the real index", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    installClipboard();
    invokeMock
      .mockResolvedValueOnce(realSummary())
      .mockRejectedValueOnce(new Error("clear denied"));
    render(<RepoIntelligencePreview />);
    await screen.findByRole("button", { name: "Clear" });
    await user.click(screen.getByRole("button", { name: "Clear" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("clear denied");
    expect(screen.getByRole("button", { name: "Clear" })).toBeVisible();
  });

  it("uses safe fallback messages for non-Error backend failures", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    installClipboard();
    invokeMock.mockRejectedValueOnce({ reason: "unstructured" });
    render(<RepoIntelligencePreview />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Saved Repo Intelligence index could not be loaded.",
    );

    invokeMock.mockRejectedValueOnce({ reason: "unstructured" });
    await user.type(screen.getByRole("textbox", { name: "Repository folder path" }), "/work/repo");
    await user.click(screen.getByRole("button", { name: "Index" }));
    const alerts = screen.getAllByRole("alert");
    expect(alerts[alerts.length - 1]).toHaveTextContent(
      "Repo Intelligence could not index that folder.",
    );
  });
});
