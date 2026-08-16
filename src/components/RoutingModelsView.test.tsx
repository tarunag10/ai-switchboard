import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { RoutingModelsView } from "./RoutingModelsView";

describe("RoutingModelsView", () => {
  it("states the explicit manual endpoint safety boundary", () => {
    const markup = renderToStaticMarkup(<RoutingModelsView hidden={false} />);

    expect(markup).toContain("Routing &amp; Models");
    expect(markup).toContain("Evidence gated");
    expect(markup).toContain("never scans your network or installs vLLM or SGLang");
    expect(markup).toContain("explicitly enable it");
    expect(markup).toContain("Model-routing experiment");
  });
});
