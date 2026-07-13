import { describe, expect, it } from "vitest";

import {
  buildRepoMapProgressSteps,
  buildRepoMapProgressSummary,
} from "./repoMapProgress";

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

  it("shows one running tool and queues later tools while generation is in progress", () => {
    const steps = buildRepoMapProgressSteps({
      generateBusy: true,
      preflightTools: [{ label: "npx", available: true }],
      toolRuns: {},
    });

    expect(steps.map((step) => step.state)).toEqual([
      "running",
      "running",
      "queued",
      "queued",
      "queued",
      "queued",
    ]);
  });

  it("advances the active tool from structured progress evidence", () => {
    const steps = buildRepoMapProgressSteps({
      generateBusy: true,
      currentToolId: "dependencyCruiser",
      toolRuns: { graphify: { status: "ok" } },
    });

    expect(steps.find((step) => step.id === "graphify")).toMatchObject({
      state: "ok",
      detail: "Completed",
    });
    expect(steps.find((step) => step.id === "dependencyCruiser")).toMatchObject({
      state: "running",
      detail: "Running",
    });
    expect(steps.find((step) => step.id === "madge")).toMatchObject({
      state: "queued",
      detail: "Queued",
    });
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

  it("keeps a retrying tool active and does not count it as complete", () => {
    const steps = buildRepoMapProgressSteps({
      generateBusy: true,
      currentToolId: "graphify",
      toolRuns: {
        graphify: { status: "retrying", detail: "attempt 1/2 failed; retrying" },
      },
    });
    expect(steps.find((step) => step.id === "graphify")).toMatchObject({
      state: "running",
      detail: "attempt 1/2 failed; retrying",
    });
    expect(
      buildRepoMapProgressSummary(steps, {
        generateBusy: true,
        currentToolId: "graphify",
        completedTools: 0,
        totalTools: 5,
        progressPercent: 0,
      }),
    ).toMatchObject({ state: "running", completed: 0, percent: 0 });
  });

  it("reports bounded aggregate progress and terminal warning state", () => {
    const steps = buildRepoMapProgressSteps({
      generateBusy: false,
      toolRuns: {
        graphify: { status: "ok" },
        madge: { status: "warning" },
      },
    });

    expect(
      buildRepoMapProgressSummary(steps, {
        generateBusy: false,
        completedTools: 2,
        totalTools: 5,
        progressPercent: 40,
      }),
    ).toEqual({
      percent: 40,
      completed: 2,
      total: 5,
      currentToolId: null,
      state: "warning",
    });
  });
});
