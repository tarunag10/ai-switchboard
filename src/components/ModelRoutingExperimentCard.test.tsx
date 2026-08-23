import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { ModelRoutingExperimentCard } from "./ModelRoutingExperimentCard";

describe("ModelRoutingExperimentCard", () => {
  it("renders observe-only defaults and the measured automatic gate", () => {
    const markup = renderToStaticMarkup(<ModelRoutingExperimentCard />);

    expect(markup).toContain("Observe only");
    expect(markup).toContain("Automatic after evidence gate");
    expect(markup).toContain("Operational routing status");
    expect(markup).toContain("automatic routing:");
    expect(markup).toContain("Disabled clients");
    expect(markup).toContain("100 samples");
    expect(markup).toContain("follow-up rework");
    expect(markup).not.toContain("Record observation");
    expect(markup).toContain("Export completion evidence");
  });
});
