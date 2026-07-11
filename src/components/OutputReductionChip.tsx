import { useState, useRef, useEffect, useId } from "react";
import type { OutputReduction } from "../lib/types";
import { percent1, compactNumber } from "../lib/dashboardHelpers";

export function OutputReductionChip({ reduction }: { reduction: OutputReduction }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const detailsId = useId();
  const isMeasured = reduction.method === "measured";

  useEffect(() => {
    if (!open) return;
    const onDown = (e: Event) => {
      if (ref.current && !ref.current.contains(e.target as Node))
        setOpen(false);
    };
    const onKey = (e: Event) => {
      if ((e as KeyboardEvent).key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="output-chip" ref={ref}>
      <button
        type="button"
        className={`output-chip__button${open ? " is-open" : ""}`}
        aria-controls={detailsId}
        aria-expanded={open}
        aria-label="Output token reduction details"
        onClick={(e) => {
          e.stopPropagation();
          setOpen((v) => !v);
        }}
        onKeyDown={(e) => e.stopPropagation()}
      >
        <span className="output-chip__dot" aria-hidden="true" />
        Output −{percent1(reduction.reductionPercent)}%
      </button>
      {open ? (
        <div
          className="output-chip__popover"
          id={detailsId}
          role="dialog"
          aria-label="Output reduction details"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="output-chip__pop-head">
            <span className="output-chip__pop-title">
              Output token reduction
            </span>
            <span className="output-chip__pop-badge">
              {isMeasured ? "measured" : "estimated"}
            </span>
          </div>
          <div className="output-chip__pop-value">
            {percent1(reduction.reductionPercent)}%
          </div>
          <dl className="output-chip__pop-stats">
            <div>
              <dt>95% CI</dt>
              <dd>
                {percent1(reduction.ciLowPercent)}–
                {percent1(reduction.ciHighPercent)}%
              </dd>
            </div>
            <div>
              <dt>Requests</dt>
              <dd>{compactNumber(reduction.requests)}</dd>
            </div>
          </dl>
          <p className="output-chip__pop-note">
            {isMeasured
              ? "Output tokens the model didn't emit because the shaper steered verbosity / routed effort down — measured against an unshaped A/B holdout."
              : "Output tokens the model didn't emit because the shaper steered verbosity / routed effort down. Output savings are counterfactual, so this is an estimate vs a learned baseline."}
          </p>
        </div>
      ) : null}
    </div>
  );
}
