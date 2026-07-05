interface SettingsUninstallCardProps {
  onOpenUninstallDialog: () => void;
}

export function SettingsUninstallCard({
  onOpenUninstallDialog,
}: SettingsUninstallCardProps) {
  return (
    <article className="soft-card panel-card">
      <div className="panel-card__header">
        <div>
          <h3>Uninstall</h3>
          <p>
            Reverses AI Switchboard for Mac changes: removes routing hooks,
            managed runtime storage, app state, login item, known Keychain
            entries, and managed config blocks. AI Switchboard for Mac will quit
            when done.
          </p>
        </div>
        <button
          className="secondary-button secondary-button--small"
          onClick={onOpenUninstallDialog}
          type="button"
        >
          Uninstall AI Switchboard for Mac
        </button>
      </div>
    </article>
  );
}
