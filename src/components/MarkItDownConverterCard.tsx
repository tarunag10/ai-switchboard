import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Copy, FileArrowUp, FolderOpen } from "@phosphor-icons/react";
import { useState } from "react";

export function MarkItDownConverterCard() {
  const [path, setPath] = useState("");
  const [markdown, setMarkdown] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const chooseDocument = async () => {
    setError(null);
    const selected = await open({ directory: false, multiple: false });
    if (typeof selected === "string") setPath(selected);
  };

  const convert = async () => {
    if (!path.trim() || busy) return;
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      setMarkdown(await invoke<string>("convert_markitdown_file", { path }));
      setNotice("Document converted locally. The source file was not modified.");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const copyMarkdown = async () => {
    if (!markdown) return;
    try {
      await navigator.clipboard.writeText(markdown);
      setNotice("Markdown copied.");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Clipboard unavailable.");
    }
  };

  return (
    <article className="soft-card panel-card" aria-labelledby="markitdown-converter-title">
      <div className="panel-card__header">
        <div>
          <h2 id="markitdown-converter-title"><FileArrowUp weight="duotone" /> Convert a document locally</h2>
          <p>MarkItDown converts a selected local document to Markdown in the managed runtime. The original remains untouched.</p>
        </div>
      </div>
      <div className="repo-intelligence-preview__controls">
        <input aria-label="Document path" value={path} onChange={(event) => setPath(event.target.value)} placeholder="/path/to/document.pdf" />
        <button type="button" className="addon-card__action" onClick={() => void chooseDocument()} disabled={busy}><FolderOpen size={15} /> Choose document</button>
        <button type="button" className="addon-card__action addon-card__action--primary" onClick={() => void convert()} disabled={busy || !path.trim()}>{busy ? "Converting…" : "Convert to Markdown"}</button>
        <button type="button" className="addon-card__action" onClick={() => void copyMarkdown()} disabled={!markdown}><Copy size={15} /> Copy Markdown</button>
      </div>
      {error ? <p role="alert">{error}</p> : null}
      {notice ? <p role="status">{notice}</p> : null}
      {markdown ? <pre aria-label="Converted Markdown" className="repo-intelligence-preview__output">{markdown}</pre> : <p className="addon-card__hint">No conversion result yet.</p>}
    </article>
  );
}
