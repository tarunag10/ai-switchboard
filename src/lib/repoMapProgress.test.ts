import { describe, expect, it } from "vitest";

import { buildRepoMapProgressSteps } from "./repoMapProgress";

describe("buildRepoMapProgressSteps", () => {
  it("marks ready preflight and pending tools before generation starts", () => {
    const steps = buildRepoMapProgressSteps({
      generateBusy: false,
      preflightTools: [{ label: "npx", available: true }],
      toolRuns: {},
    });

    expect(steps[0]).toMatchObject({ id: "preflight", state: "ok", detail: "Tooling ready" });
    expect(steps.find((step) => step.id === "graphify")).toMatchObject({
      state: "pending",
      detail: "Waiting",
    });
  });

  it("surfaces missing preflight tools before a long map run", () => {
    const steps = buildRepoMapProgressSteps({
      generateBusy: false,
      preflightTools: [
        { label: "uv", available: false, installHint: "brew install uv" },
        { label: "Graphviz", available: false, installHint: "brew install graphviz" },
      ],
      toolRuns: {},
    });

    expect(steps[0]).toMatchObject({ state: "warning", detail: "2 missing tools" });
  });

  it("shows queued/running tool states while generation is in progress", () => {
    const steps = buildRepoMapProgressSteps({
      generateBusy: true,
      preflightTools: [{ label: "npx", available: true }],
      toolRuns: {},
    });

    expect(steps.map((step) => step.state)).toEqual([
      "running",
      "running",
      "running",
      "running",
      "running",
      "running",
    ]);
  });

  it("keeps partial Graphify success visible as a warning", () => {
    const steps = buildRepoMapProgressSteps({
      generateBusy: false,
      toolRuns: {
        graphify: {
          status: "warning",
          detail: "Exited 1.",
          remediation: "Graphify wrote graph JSON; inspect semantic extraction later.",
        },
        madge: { status: "ok", detail: "Completed.", remediation: null },
      },
    });

    expect(steps.find((step) => step.id === "graphify")).toMatchObject({
      state: "warning",
      detail: "Graphify wrote graph JSON; inspect semantic extraction later.",
    });
    expect(steps.find((step) => step.id === "madge")).toMatchObject({
      state: "ok",
      detail: "Completed.",
    });
  });

  it("marks unfinished tools as errors when generation fails before producing runs", () => {
    const steps = buildRepoMapProgressSteps({
      generateBusy: false,
      generateError: "Failed to start repo map generator",
      toolRuns: {},
    });

    expect(steps.map((step) => step.state)).toEqual(["error", "error", "error", "error", "error", "error"]);
  });
});
