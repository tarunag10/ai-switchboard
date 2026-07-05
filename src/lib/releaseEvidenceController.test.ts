import { describe, expect, it, vi } from "vitest";

import {
  runLocalReleaseEvidenceSequence,
  runReleaseEvidenceCommand,
  type ReleaseEvidenceCommandResult,
  type ReleaseEvidenceControllerOptions,
  type ReleaseEvidenceInvoke,
  type ReleaseReadinessReportPayload,
} from "./releaseEvidenceController";
import { localReleaseEvidenceCommandIds } from "./releaseReadiness";

const commandResult: ReleaseEvidenceCommandResult = {
  commandId: "rollback-center-validation",
  label: "Rollback Center validation",
  command: "npm run smoke:rollback:local",
  summaryPath: "dist/local-rollback-validation-summary.md",
  stdoutTail: "ok",
  stderrTail: "",
  stdout: "ok",
  stderr: "",
};

const reportPayload: ReleaseReadinessReportPayload = {
  reportPath: "dist/release-readiness-report.md",
  report: null,
};

function options(invoke: ReleaseEvidenceInvoke) {
  return {
    invoke,
    setBusyId: vi.fn(),
    setCopyNotice: vi.fn(),
    setError: vi.fn(),
    setReport: vi.fn(),
    setResult: vi.fn(),
    setTimeout: vi.fn((handler: () => void) => handler()),
  } satisfies ReleaseEvidenceControllerOptions;
}

describe("releaseEvidenceController", () => {
  it("runs a single evidence command and refreshes release readiness", async () => {
    const invoke = vi.fn(async (command: string) => {
      if (command === "run_release_evidence_command") return commandResult;
      if (command === "load_release_readiness_report") return reportPayload;
      throw new Error(`unexpected command ${command}`);
    }) as ReleaseEvidenceInvoke;
    const setup = options(invoke);

    await runReleaseEvidenceCommand("rollback-center-validation", setup);

    expect(setup.setBusyId).toHaveBeenNthCalledWith(
      1,
      "rollback-center-validation",
    );
    expect(invoke).toHaveBeenCalledWith("run_release_evidence_command", {
      commandId: "rollback-center-validation",
    });
    expect(setup.setResult).toHaveBeenCalledWith(commandResult);
    expect(setup.setCopyNotice).toHaveBeenCalledWith(
      "Rollback Center validation evidence generated.",
    );
    expect(invoke).toHaveBeenCalledWith("load_release_readiness_report");
    expect(setup.setReport).toHaveBeenCalledWith(reportPayload);
    expect(setup.setBusyId).toHaveBeenLastCalledWith(null);
  });

  it("surfaces a single command failure and clears busy state", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("not installed");
    }) as ReleaseEvidenceInvoke;
    const setup = options(invoke);

    await runReleaseEvidenceCommand("install-smoke", setup);

    expect(setup.setError).toHaveBeenCalledWith("not installed");
    expect(setup.setBusyId).toHaveBeenLastCalledWith(null);
    expect(setup.setReport).not.toHaveBeenCalled();
  });

  it("runs the local evidence sequence and refreshes once at the end", async () => {
    const commandIds: string[] = [];
    const invoke = vi.fn(async (command: string, args?: Record<string, unknown>) => {
      if (command === "run_release_evidence_command") {
        commandIds.push(String(args?.commandId));
        return { ...commandResult, commandId: String(args?.commandId) };
      }
      if (command === "load_release_readiness_report") return reportPayload;
      throw new Error(`unexpected command ${command}`);
    }) as ReleaseEvidenceInvoke;
    const setup = options(invoke);

    await runLocalReleaseEvidenceSequence(setup);

    expect(setup.setBusyId).toHaveBeenNthCalledWith(1, "local-evidence");
    expect(commandIds).toEqual(localReleaseEvidenceCommandIds);
    expect(setup.setReport).toHaveBeenCalledWith(reportPayload);
    expect(setup.setCopyNotice).toHaveBeenLastCalledWith(null);
    expect(setup.setBusyId).toHaveBeenLastCalledWith(null);
  });
});
