import { FileArrowUp, ShieldCheck } from "@phosphor-icons/react";
import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import {
  replayRedactedRouteEvents,
  type OssHarnessReplayResult,
} from "../lib/ossHarnessReplay";

export function OssHarnessReplayPanel() {
  const [result, setResult] = useState<OssHarnessReplayResult | null>(null);
  const [path, setPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function chooseReplay() {
    setBusy(true);
    setError(null);
    try {
      const selected = await open({
        multiple: false,
        title: "Choose redacted OSS harness replay",
        filters: [{ name: "JSON replay", extensions: ["json"] }],
      });
      if (typeof selected !== "string") return;
      setPath(selected);
      setResult(await replayRedactedRouteEvents(selected));
    } catch (reason) {
      setResult(null);
      setError(reason instanceof Error ? reason.message : "Replay could not be validated.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <article className="soft-card panel-card" aria-labelledby="oss-replay-title">
      <div className="panel-card__header">
        <div>
          <h2 id="oss-replay-title">Redacted harness replay</h2>
          <p>
            Validate a local route-event replay without provider traffic. Prompts, responses,
            headers, credentials, and automatic promotion are rejected or disabled.
          </p>
        </div>
        <ShieldCheck weight="duotone" aria-hidden="true" />
      </div>
      <button className="secondary-button secondary-button--small" type="button" onClick={() => void chooseReplay()} disabled={busy}>
        <FileArrowUp weight="bold" size={13} aria-hidden="true" />
        {busy ? "Validating…" : "Choose replay JSON"}
      </button>
      {path ? <p className="optimize-minimal__meta">Validated: {path}</p> : null}
      {error ? <p className="addons__error" role="alert">{error}</p> : null}
      {result ? (
        <div className="optimization-evidence-capture__grid" role="status">
          <p><strong>{result.eventCount}</strong> events · p95 latency {result.latency.p95Ms ?? "unavailable"} ms</p>
          <p>Mode: observe-only · provider traffic: none · automatic promotion: disabled</p>
          <p className="optimize-minimal__meta">Digest: {result.replayDigest}</p>
        </div>
      ) : null}
    </article>
  );
}
