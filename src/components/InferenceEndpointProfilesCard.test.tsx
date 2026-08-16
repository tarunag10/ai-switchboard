import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { InferenceEndpointProfilesCard } from "./InferenceEndpointProfilesCard";

describe("InferenceEndpointProfilesCard", () => {
  it("keeps enrollment and selection boundaries explicit before configuration", () => {
    const markup = renderToStaticMarkup(<InferenceEndpointProfilesCard />);

    expect(markup).toContain("Add to allowlist");
    expect(markup).toContain("Selection stays manual");
    expect(markup).toContain("never rewrites coding-client config");
    expect(markup).toContain("llama.cpp local profile");
    expect(markup).toContain("LiteLLM externally owned profile");
    expect(markup).toContain("Envoy AI Gateway enterprise profile");
    expect(markup).toContain("NVIDIA Dynamo deployment profile");
    expect(markup).toContain("TensorRT-LLM verified endpoint");
    expect(markup).toContain("No user-managed endpoints configured");
  });
});
