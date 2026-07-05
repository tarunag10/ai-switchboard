import { describeInvokeError } from "./appHelpers";
import { localReleaseEvidenceCommandIds } from "./releaseReadiness";
import type { ReleaseReadinessReportSnapshot } from "./releaseReadiness";

export interface ReleaseReadinessReportPayload {
  reportPath: string;
  report: ReleaseReadinessReportSnapshot | null;
}

export interface ReleaseEvidenceCommandResult {
  commandId: string;
  label: string;
  command: string;
  summaryPath: string | null;
  stdoutTail: string;
  stderrTail: string;
  stdout: string;
  stderr: string;
}

export type ReleaseEvidenceInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

export interface ReleaseEvidenceControllerOptions {
  invoke: ReleaseEvidenceInvoke;
  setBusyId: (id: string | null) => void;
  setCopyNotice: (notice: string | null) => void;
  setError: (message: string | null) => void;
  setReport: (payload: ReleaseReadinessReportPayload) => void;
  setResult: (result: ReleaseEvidenceCommandResult) => void;
  setTimeout: (handler: () => void, timeout: number) => unknown;
}

export async function runReleaseEvidenceCommand(
  commandId: string,
  options: ReleaseEvidenceControllerOptions,
) {
  options.setBusyId(commandId);
  options.setError(null);
  options.setCopyNotice(null);
  try {
    const result = await options.invoke<ReleaseEvidenceCommandResult>(
      "run_release_evidence_command",
      { commandId },
    );
    options.setResult(result);
    options.setCopyNotice(`${result.label} evidence generated.`);
    options.setTimeout(() => options.setCopyNotice(null), 2500);
    await refreshReleaseReadinessReport(options);
  } catch (error) {
    options.setError(
      describeInvokeError(error, "Could not run release evidence command."),
    );
  } finally {
    options.setBusyId(null);
  }
}

export async function runLocalReleaseEvidenceSequence(
  options: ReleaseEvidenceControllerOptions,
) {
  options.setBusyId("local-evidence");
  options.setError(null);
  options.setCopyNotice("Running local release evidence...");
  try {
    let lastResult: ReleaseEvidenceCommandResult | null = null;
    for (const commandId of localReleaseEvidenceCommandIds) {
      const result = await options.invoke<ReleaseEvidenceCommandResult>(
        "run_release_evidence_command",
        { commandId },
      );
      lastResult = result;
      options.setResult(result);
      options.setCopyNotice(`${result.label} evidence generated.`);
    }
    await refreshReleaseReadinessReport(options);
    options.setCopyNotice(
      lastResult
        ? "Local release evidence sequence completed."
        : "No local evidence commands ran.",
    );
    options.setTimeout(() => options.setCopyNotice(null), 3000);
  } catch (error) {
    options.setError(
      describeInvokeError(error, "Could not run local release evidence."),
    );
  } finally {
    options.setBusyId(null);
  }
}

async function refreshReleaseReadinessReport(
  options: ReleaseEvidenceControllerOptions,
) {
  const payload = await options.invoke<ReleaseReadinessReportPayload>(
    "load_release_readiness_report",
  );
  options.setReport(payload);
}
