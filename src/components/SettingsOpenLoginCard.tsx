interface SettingsOpenLoginCardProps {
  autostartEnabled: boolean | null;
  autostartBusy: boolean;
  onToggle: (enabled: boolean) => void;
}

export function SettingsOpenLoginCard({
  autostartEnabled,
  autostartBusy,
  onToggle,
}: SettingsOpenLoginCardProps) {
  return (
    <div className="soft-card panel-card">
      <h3>Open on login</h3>
      <p>
        Automatically launch AI Switchboard for Mac whenever you log in or
        restart.
      </p>
      <div className="connector-item__controls">
        <button
          aria-checked={autostartEnabled === true}
          aria-label={`${autostartEnabled ? "Disable" : "Enable"} open on login`}
          className={`connector-switch${autostartEnabled ? " is-on" : ""}`}
          disabled={autostartBusy || autostartEnabled === null}
          onClick={() => onToggle(!autostartEnabled)}
          role="switch"
          type="button"
        >
          <span className="connector-switch__thumb" />
        </button>
      </div>
    </div>
  );
}
