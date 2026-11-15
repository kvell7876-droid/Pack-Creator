import { useEffect, useRef, useState } from 'preact/hooks';
import { api } from './api';
import * as ide from './editor';
import { messageOf, notify } from './notify';
import type { FileVersion } from './types';

const when = (id: number) => {
  const date = new Date(id);
  const seconds = (Date.now() - id) / 1000;
  const ago = seconds < 90 ? 'just now' : seconds < 5400 ? `${Math.round(seconds / 60)} min ago` : seconds < 172800 ? `${Math.round(seconds / 3600)} h ago` : `${Math.round(seconds / 86400)} days ago`;

  return { clock: date.toLocaleString(), ago };
};

/**
 * Earlier versions of a file: the list on the left, and the chosen one next to what the file is now (the old text on the left, now on the right). Restore puts the old text
 * back as one edit, so it can be undone like any other.
 */
export function HistoryDialog({ rel, current, onRestore, onClose }: { rel: string; current: string; onRestore: (text: string) => void; onClose: () => void }) {
  const [versions, setVersions] = useState<FileVersion[] | null>(null);
  const [chosen, setChosen] = useState<number | null>(null);
  const [old, setOld] = useState<string | null>(null);
  const host = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void api.fileHistory(rel).then(list => { setVersions(list); setChosen(list[0]?.id ?? null); }).catch(error => notify(messageOf(error)));
  }, [rel]);

  useEffect(() => {
    if (chosen === null) { setOld(null); return; }

    let live = true;

    void api.readVersion(rel, chosen).then(text => { if (live) setOld(text); }).catch(error => notify(messageOf(error)));

    return () => { live = false; };
  }, [rel, chosen]);

  /* The comparison: models of its own, so the file's editor is not touched. */
  useEffect(() => {
    if (old === null || !host.current) return;

    const diff = ide.monaco.editor.createDiffEditor(host.current, { theme: 'ph-dark', readOnly: true, automaticLayout: true, renderSideBySide: true, fontSize: 12, scrollBeyondLastLine: false, minimap: { enabled: false } });
    const before = ide.monaco.editor.createModel(old, 'json');
    const after = ide.monaco.editor.createModel(current, 'json');

    diff.setModel({ original: before, modified: after });

    return () => { diff.dispose(); before.dispose(); after.dispose(); };
  }, [old]);

  const name = rel.split('/').pop();

  return (
    <div class="scrim" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }} onKeyDown={event => { if (event.key === 'Escape') onClose(); }}>
      <div class="dialog wide" role="dialog" aria-label="History">
        <h3>Earlier versions of {name}</h3>

        {versions && versions.length === 0 && <p class="muted" style="margin:0">No earlier version is kept yet. A copy is kept as you work, at most one every half minute.</p>}

        {versions && versions.length > 0 && (
          <div class="history">
            <div class="history-list">
              {versions.map(v => {
                const t = when(v.id);

                return (
                  <button key={v.id} class={'history-item' + (chosen === v.id ? ' on' : '')} onClick={() => setChosen(v.id)}>
                    <div>{t.ago}</div>
                    <div class="muted" style="font-size:11px">{t.clock} · {(v.size / 1024).toFixed(1)} KiB</div>
                  </button>
                );
              })}
            </div>
            <div>
              <div class="muted" style="margin-bottom:6px">The old version is on the left, the file as it is now on the right.</div>
              <div class="history-diff" ref={host} />
            </div>
          </div>
        )}

        <div class="foot">
          <button class="btn ghost" onClick={onClose}>Close</button>
          <button class="btn primary" disabled={old === null || old === current} onClick={() => old !== null && onRestore(old)}>Restore this version</button>
        </div>
      </div>
    </div>
  );
}
