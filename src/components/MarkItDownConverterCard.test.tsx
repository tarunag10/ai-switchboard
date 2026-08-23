import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MarkItDownConverterCard } from "./MarkItDownConverterCard";

const { invoke, open } = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open }));

describe("MarkItDownConverterCard", () => {
  beforeEach(() => {
    invoke.mockReset();
    open.mockReset();
    open.mockResolvedValue("/tmp/report.pdf");
    invoke.mockResolvedValue("# Converted report\n");
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
  });

  it("chooses, converts, and copies a local document", async () => {
    render(<MarkItDownConverterCard />);
    fireEvent.click(screen.getByRole("button", { name: "Choose document" }));
    await waitFor(() => expect(screen.getByRole("textbox", { name: "Document path" })).toHaveValue("/tmp/report.pdf"));
    fireEvent.click(screen.getByRole("button", { name: "Convert to Markdown" }));
    expect(await screen.findByRole("status")).toHaveTextContent("converted locally");
    expect(invoke).toHaveBeenCalledWith("convert_markitdown_file", { path: "/tmp/report.pdf" });
    fireEvent.click(screen.getByRole("button", { name: "Copy Markdown" }));
    expect(await screen.findByRole("status")).toHaveTextContent("Markdown copied");
  });

  it("surfaces conversion failures without hiding the selected path", async () => {
    invoke.mockRejectedValue(new Error("MarkItDown unavailable"));
    render(<MarkItDownConverterCard />);
    fireEvent.change(screen.getByRole("textbox", { name: "Document path" }), { target: { value: "/tmp/report.pdf" } });
    fireEvent.click(screen.getByRole("button", { name: "Convert to Markdown" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("MarkItDown unavailable");
    expect(screen.getByRole("textbox", { name: "Document path" })).toHaveValue("/tmp/report.pdf");
  });
});
