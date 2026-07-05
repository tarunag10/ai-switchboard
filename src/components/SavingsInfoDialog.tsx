type SavingsInfoDialogProps = {
  minimumEstimatedSavingsLabel: string;
  onClose: () => void;
};

export function SavingsInfoDialog({
  minimumEstimatedSavingsLabel,
  onClose,
}: SavingsInfoDialogProps) {
  return (
    <div
      className="modal-backdrop"
      role="dialog"
      aria-modal="true"
      onClick={onClose}
    >
      <div className="modal-card" onClick={(event) => event.stopPropagation()}>
        <h3>How savings are calculated</h3>
        <p>
          Headroom intercepts and prunes all inputs before sending them to
          Claude or Codex.
        </p>
        <p>Savings = tokens removed &times; API token prices.</p>
        <p>This is an optimistic estimate.</p>
        <p>
          Without Headroom, when tokens are sent to Claude for the first time
          they would be stored in their cache. Once in the cache, whenever these
          same tokens are sent again Claude applies a 90% discount to their
          cost. In our testing, this can reduce the actual savings by at most
          50%.
        </p>
        <p>
          Even accounting for caching, you've likely saved at least{" "}
          <strong>{minimumEstimatedSavingsLabel}</strong>.
        </p>
        <div className="modal-actions">
          <button
            className="button button--primary"
            onClick={onClose}
            type="button"
          >
            Got it
          </button>
        </div>
      </div>
    </div>
  );
}
