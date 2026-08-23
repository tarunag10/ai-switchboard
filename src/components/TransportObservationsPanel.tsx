import { ArrowClockwise, Pulse } from "@phosphor-icons/react";
import { useEffect, useMemo, useState } from "react";
import {
  loadTransportObservations,
  type TransportObservation,
} from "../lib/transportObservations";

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function duration(observation: TransportObservation): string {
  if (observation.completedAtMs === null) return "In progress";
  const elapsed = Math.max(0, observation.completedAtMs - observation.startedAtMs);
  return `${elapsed} ms`;
}

function outcomeClass(outcome: TransportObservation["terminalOutcome"]): string {
  if (outcome === "success") return "success";
  if (outcome === null) return "pending";
  return "failure";
}

export function TransportObservationsPanel() {
  const [observations, setObservations] = useState<TransportObservation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      setObservations(await loadTransportObservations());
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Transport telemetry is unavailable.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  const summary = useMemo(() => {
    const completed = observations.filter((item) => item.completedAtMs !== null);
    const failures = completed.filter((item) => item.terminalOutcome !== "success");
    return { completed: completed.length, failures: failures.length };
  }, [observations]);

  return (
    <section className="optimize-minimal transport-observations" aria-labelledby="transport-observations-title">
      <div className="optimize-card__head">
        <div className="optimize-card__title-row">
          <span className="optimize-card__title-icon" aria-hidden="true">
            <Pulse weight="duotone" />
          </span>
          <div>
            <h2 id="transport-observations-title">Transport observations</h2>
            <p className="optimize-minimal__meta">
              Content-free local route and outcome telemetry; request bodies and secrets are not retained.
            </p>
          </div>
        </div>
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void refresh()}
          disabled={loading}
        >
          <ArrowClockwise weight="bold" size={12} aria-hidden="true" />
          {loading ? "Refreshing" : "Refresh"}
        </button>
      </div>

      {error ? <p className="install-progress__error" role="alert">{error}</p> : null}
      {!error && observations.length === 0 ? (
        <p className="loading-copy">{loading ? "Loading transport telemetry…" : "No transport observations yet."}</p>
      ) : null}
      {observations.length > 0 ? (
        <>
          <p className="optimize-minimal__meta" role="status">
            {observations.length} recorded · {summary.completed} completed · {summary.failures} failed
          </p>
          <ul className="transport-observations__list">
            {observations.slice(-8).reverse().map((observation) => (
              <li className="transport-observations__item" key={observation.eventId}>
                <div>
                  <strong>{label(observation.route)}</strong>
                  <span>{label(observation.requestClass)} · {observation.streaming ? "Streaming" : "Unary"}</span>
                </div>
                <div className={`transport-observations__outcome transport-observations__outcome--${outcomeClass(observation.terminalOutcome)}`}>
                  <span>{observation.terminalOutcome ? label(observation.terminalOutcome) : "In progress"}</span>
                  <small>{duration(observation)}{observation.statusCode ? ` · HTTP ${observation.statusCode}` : ""}</small>
                </div>
              </li>
            ))}
          </ul>
        </>
      ) : null}
    </section>
  );
}
