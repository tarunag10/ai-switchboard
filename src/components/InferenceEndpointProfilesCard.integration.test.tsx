import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { InferenceEndpointProfilesCard } from "./InferenceEndpointProfilesCard";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

const endpoint = {
  id: "studio", label: "Studio GPU", locationClass: "local_network", enabled: true, selected: false,
  verification: { status: "verified", runtimeId: "vllm", runtimeVersion: "1.0" },
  baseUrl: "http://192.168.1.50:8000/v1", host: "192.168.1.50", modelId: "Qwen/Qwen3-Coder",
  maxContext: 32768, quantization: null, runtimeKind: "vllm", externallyOwned: false, remoteConnectivityOptIn: false,
};
const snapshot = { diagnostics: [endpoint], selectedEndpointId: null };

describe("InferenceEndpointProfilesCard native workflows", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.restoreAllMocks();
  });

  it("verifies, selects, and disables an endpoint with exact restart payloads", async () => {
    const user = userEvent.setup();
    invokeMock.mockImplementation((command: string) => command === "list_inference_endpoints" ? Promise.resolve(snapshot) : Promise.resolve(undefined));
    render(<InferenceEndpointProfilesCard />);
    expect(await screen.findByText("Studio GPU")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Verify" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("verify_inference_endpoint", { endpointId: "studio" }));
    expect(await screen.findByText("Endpoint verification evidence refreshed.")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Select & restart" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("select_inference_endpoint", { endpointId: "studio", restartOptimizer: true }));
    expect(await screen.findByText("Endpoint selected and optimizer restarted.")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Disable" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("disable_inference_endpoint", { endpointId: "studio", restartOptimizer: true }));
    expect(await screen.findByText("Endpoint disabled; coding-client config was unchanged.")).toBeInTheDocument();
  });

  it("adds a typed endpoint only after confirmation and normalizes numeric context", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "prompt").mockReturnValue("ADD ENDPOINT local-gpu");
    invokeMock.mockImplementation((command: string) => {
      if (command === "list_inference_endpoints") return Promise.resolve({ diagnostics: [], selectedEndpointId: null });
      if (command === "add_inference_endpoint") return Promise.resolve(snapshot);
      return Promise.resolve(undefined);
    });
    render(<InferenceEndpointProfilesCard />);
    await screen.findByText("No user-managed endpoints configured.");
    await user.selectOptions(screen.getByLabelText("Endpoint runtime type"), "tensorrt_llm");
    await user.type(screen.getByLabelText("Endpoint ID"), " local-gpu ");
    await user.type(screen.getByLabelText("Display label"), "Local GPU");
    await user.type(screen.getByLabelText("Base URL"), "http://127.0.0.1:8000/v1");
    await user.type(screen.getByLabelText("Model ID"), "Qwen3");
    await user.type(screen.getByLabelText("Max context (optional)"), "32768");
    await user.type(screen.getByLabelText("Quantization (optional)"), "Q4_K_M");
    await user.click(screen.getByRole("button", { name: "Add to allowlist" }));

    expect(window.prompt).toHaveBeenCalledWith(expect.stringContaining("ADD ENDPOINT local-gpu"));
    expect(invokeMock).toHaveBeenCalledWith("add_inference_endpoint", {
      input: {
        id: " local-gpu ", label: "Local GPU", baseUrl: "http://127.0.0.1:8000/v1", modelId: "Qwen3",
        maxContext: 32768, quantization: "Q4_K_M", remoteConnectivityOptIn: false, kind: "tensorrt_llm",
      },
      confirmation: "ADD ENDPOINT local-gpu",
    });
    expect(await screen.findByText(/Endpoint added to the local allowlist/)).toBeInTheDocument();
  });

  it("does not invoke add after cancellation and surfaces refresh/action errors", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "prompt").mockReturnValue(null);
    invokeMock.mockResolvedValueOnce({ diagnostics: [], selectedEndpointId: null });
    const view = render(<InferenceEndpointProfilesCard />);
    await screen.findByText("No user-managed endpoints configured.");
    await user.type(screen.getByLabelText("Endpoint ID"), "cancelled");
    await user.type(screen.getByLabelText("Display label"), "Cancelled");
    await user.type(screen.getByLabelText("Base URL"), "http://127.0.0.1:8000/v1");
    await user.type(screen.getByLabelText("Model ID"), "Qwen3");
    await user.click(screen.getByRole("button", { name: "Add to allowlist" }));
    expect(invokeMock).toHaveBeenCalledTimes(1);

    view.unmount();
    invokeMock.mockReset().mockRejectedValueOnce(new Error("endpoint service offline"));
    render(<InferenceEndpointProfilesCard />);
    expect(await screen.findByRole("alert")).toHaveTextContent("endpoint service offline");
  });
});
