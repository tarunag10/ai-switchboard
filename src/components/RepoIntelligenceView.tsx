import type { RepoIntelligenceSummary } from "../lib/repoIntelligence";
import type { RuntimeStatus } from "../lib/types";
import { RepoIntelligencePreview } from "./RepoIntelligencePreview";

interface RepoIntelligenceViewProps {
  hidden?: boolean;
  runtimeStatus: RuntimeStatus | null;
  onSummaryChange: (summary: RepoIntelligenceSummary) => void;
}

export function RepoIntelligenceView({
  hidden,
  runtimeStatus,
  onSummaryChange,
}: RepoIntelligenceViewProps) {
  const headroomHealthy =
    runtimeStatus?.proxyReachable === true &&
    runtimeStatus.running === true &&
    runtimeStatus.paused === false;
  const rtkHealthy = runtimeStatus?.rtk.installed === true && runtimeStatus.rtk.enabled === true;

  return (
    <div className="tray-content tray-content--repo-intelligence" hidden={hidden}>
      <section className="repo-intelligence-view">
        <header className="repo-intelligence-view__header">
          <div>
            <h1>Repo Intelligence</h1>
            <p className="repo-intelligence-view__subtitle">
              Index a local repository, review graph signals, and copy bounded context packs for
              coding agents.
            </p>
          </div>
          <span className="repo-intelligence-view__badge">Local only</span>
        </header>
        <RepoIntelligencePreview
          headroomHealthy={headroomHealthy}
          onSummaryChange={onSummaryChange}
          rtkHealthy={rtkHealthy}
        />
      </section>
    </div>
  );
}
