import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { RoutingModelsView } from "./RoutingModelsView";

describe("RoutingModelsView", () => {
  it("states the explicit manual endpoint safety boundary", () => {
    const markup = renderToStaticMarkup(<RoutingModelsView hidden={false} />);

    expect(markup).toContain("Routing &amp; Models");
    expect(markup).toContain("Manual only");
    expect(markup).toContain("never scans your network or installs vLLM");
    expect(markup).toContain("explicitly enable it");
  });
});
