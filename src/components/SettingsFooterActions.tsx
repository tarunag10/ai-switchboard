import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

interface SettingsFooterActionsProps {
  supportUrl: string;
}

export function SettingsFooterActions({
  supportUrl,
}: SettingsFooterActionsProps) {
  const [error, setError] = useState<string | null>(null);

  async function openSupport() {
    setError(null);
    try {
      await invoke("open_external_link", { url: supportUrl });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not open support.");
    }
  }

  async function quitApp() {
    setError(null);
    try {
      await invoke("quit_headroom");
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : "Could not quit Mac AI Switchboard.",
      );
    }
  }

  return (
    <>
      <button
        className="contact-link"
        onClick={() => void openSupport()}
        type="button"
      >
        Support
      </button>
      <button
        className="quit-button"
        onClick={() => void quitApp()}
        type="button"
      >
        Quit Mac AI Switchboard
      </button>
      {error ? <p className="runtime-status__error">{error}</p> : null}
    </>
  );
}
