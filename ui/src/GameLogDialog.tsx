import { useEffect, useState } from 'preact/hooks';
import { api, pickFile } from './api';
import { Dialog } from './Dialogs';
import { messageOf } from './notify';
import type { GameLogView } from './types';

const STORE = 'ph.gamelog.path';

const remembered = () => {
  try { return localStorage.getItem(STORE) ?? ''; } catch { return ''; }
};

const age = (seconds: number | null) => {
  if (seconds === null) return '';
  if (seconds < 90) return 'just now';
  if (seconds < 5400) return `${Math.round(seconds / 60)} minutes ago`;
  if (seconds < 172800) return `${Math.round(seconds / 3600)} hours ago`;

  return `${Math.round(seconds / 86400)} days ago`;
};

/**
 * What the game said on its last run about Pleasure Horizons and Strut: a girl, scene, trait or dialogue file it could not accept, and what asset sync offered. Each message
 * that names a file of this pack has a button that opens the file. After "Try in a world" and a /reload, press Refresh.
 */
export function GameLogDialog({ onOpen, onClose }: { onOpen: (rel: string) => void; onClose: () => void }) {
  const [path, setPath] = useState(remembered());
  const [view, setView] = useState<GameLogView | null>(null);
  const [problem, setProblem] = useState('');
  const [onlyPack, setOnlyPack] = useState(false);
  const [warnings, setWarnings] = useState(true);

  const load = async (target: string) => {
    setProblem('');

    try {
      const found = await api.readGameLog(target);

      setView(found);

      try { localStorage.setItem(STORE, target); } catch { /* the choice is just not remembered */ }
    } catch (error) {
      setView(null);
      setProblem(messageOf(error));
    }
  };

  useEffect(() => {
    void (async () => {
      const start = path || (await api.gameLogDefault().catch(() => null)) || '';

      if (start) {
        setPath(start);
        await load(start);
      } else {
        setProblem('The game\'s log was not found in the usual place. Choose it with Browse (it is logs\\latest.log in the game folder).');
      }
    })();
  }, []);

  const browse = async () => {
    const chosen = await pickFile("Choose the game's log (latest.log)", ['log']);

    if (chosen) {
      setPath(chosen);
      await load(chosen);
    }
  };

  const shown = (view?.entries ?? []).filter(e => (warnings || e.level !== 'WARN') && (!onlyPack || e.file));
  const errors = (view?.entries ?? []).filter(e => e.level !== 'WARN').length;

  return (
    <Dialog title="The game's log" onClose={onClose}>
      <div class="field">
        <label>The game's log file</label>
        <div class="row">
          <input class="input grow" value={path} onInput={e => setPath((e.target as HTMLInputElement).value)} onKeyDown={e => { if (e.key === 'Enter') void load(path.trim()); }} />
          <button class="btn" onClick={() => void browse()}>Browse…</button>
          <button class="btn" onClick={() => void load(path.trim())}>Refresh</button>
        </div>
      </div>

      {problem && <div class="form-note">{problem}</div>}

      {view && (
        <>
          <div class="muted" style="margin-bottom:8px">
            Written {age(view.ageSeconds)}.
            {view.offer
              ? ` Asset sync offered ${view.offer.files} file${view.offer.files === 1 ? '' : 's'} (${(view.offer.bytes / 1024).toFixed(0)} KiB) at ${view.offer.time}; this pack has ${view.packFiles} under asset_sync (the game offers every pack's files together).`
              : ' Asset sync has not said what it offers in this log yet.'}
          </div>

          <div class="row" style="margin-bottom:8px">
            <span class={errors ? 'chip bad' : 'chip'}>{errors} error{errors === 1 ? '' : 's'}</span>
            <label class="check-inline"><input type="checkbox" checked={warnings} onChange={e => setWarnings((e.target as HTMLInputElement).checked)} /> warnings</label>
            <label class="check-inline"><input type="checkbox" checked={onlyPack} onChange={e => setOnlyPack((e.target as HTMLInputElement).checked)} /> only those about files of this pack</label>
          </div>

          <div class="log-list">
            {shown.length === 0 && <div class="muted">Nothing about Pleasure Horizons or Strut{onlyPack ? ' and this pack' : ''} in this log. That is good news.</div>}
            {shown.map((entry, i) => (
              <div class={'log-entry ' + entry.level.toLowerCase()} key={i}>
                <div class="log-head">
                  <span class="muted">{entry.time}</span>
                  <span class={'chip' + (entry.level === 'WARN' ? '' : ' bad')}>{entry.level.toLowerCase()}</span>
                  <span class="muted">{entry.source}</span>
                  {entry.count > 1 && <span class="muted">x{entry.count}</span>}
                  {entry.file && <button class="btn ghost small" style="margin-left:auto" onClick={() => { onClose(); onOpen(entry.file!); }}>Open {entry.file.split('/').pop()}</button>}
                </div>
                <div class="log-text">{entry.message.trim()}</div>
              </div>
            ))}
          </div>
        </>
      )}

      <div class="foot">
        <button class="btn ghost" onClick={onClose}>Close</button>
      </div>
    </Dialog>
  );
}
