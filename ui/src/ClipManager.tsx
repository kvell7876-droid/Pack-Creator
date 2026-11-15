import { useState } from 'preact/hooks';
import { api } from './api';
import { Dialog } from './Dialogs';
import { messageOf, notify } from './notify';
import type { ClipData } from './types';

/** Lists the clips of an animation file so the unwanted ones can be removed. A copy of the file as it was is kept in the project's backups. */
export function ClipManager({ rel, clips, onClose, onDeleted }: { rel: string; clips: ClipData[]; onClose: () => void; onDeleted: () => void }) {
  const [marked, setMarked] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState('');
  const [busy, setBusy] = useState(false);

  const shown = clips.filter(c => c.name.toLowerCase().includes(filter.trim().toLowerCase()));
  const toggle = (name: string) => setMarked(previous => { const next = new Set(previous); if (next.has(name)) next.delete(name); else next.add(name); return next; });
  const all = marked.size === clips.length;

  const remove = async () => {
    setBusy(true);

    try {
      await api.deleteClips(rel, [...marked]);
      notify(`Removed ${marked.size} animation${marked.size === 1 ? '' : 's'}. A copy of the file as it was is in the project's .packcreator/backups folder.`, true);
      onDeleted();
    } catch (error) {
      notify(messageOf(error));
      setBusy(false);
    }
  };

  return (
    <Dialog title="Animations in this file" onClose={onClose}>
      <p class="muted" style="margin-top:0">{rel.slice(rel.lastIndexOf('/') + 1)}: {clips.length} animations. Tick the ones to remove.</p>
      <div class="row" style="gap:6px;margin-bottom:6px">
        <input class="input grow" value={filter} placeholder="Search…" onInput={e => setFilter((e.target as HTMLInputElement).value)} />
        <button class="btn ghost small" onClick={() => setMarked(new Set(shown.map(c => c.name)))}>Tick {filter.trim() ? 'these' : 'all'}</button>
        <button class="btn ghost small" onClick={() => setMarked(new Set())}>Tick none</button>
      </div>
      <div class="clip-manager">
        {shown.map(c => (
          <label class={'clip-manager-row' + (marked.has(c.name) ? ' marked' : '')} key={c.name}>
            <input type="checkbox" checked={marked.has(c.name)} onChange={() => toggle(c.name)} />
            <span class="row-name">{c.name.replace(/^animation\./, '')}</span>
            <span class="muted">{c.duration.toFixed(2)} s{c.events.length > 0 ? `, ${c.events.length} cue${c.events.length === 1 ? '' : 's'}` : ''}</span>
          </label>
        ))}
        {shown.length === 0 && <span class="muted">Nothing matches.</span>}
      </div>
      {all && <div class="hint" style="color:var(--warning)">A file keeps at least one animation. To remove them all, delete the file instead.</div>}
      <div class="foot">
        <button class="btn ghost" onClick={onClose}>Close</button>
        <button class="btn primary" disabled={marked.size === 0 || all || busy} onClick={() => void remove()}>{busy ? 'Removing…' : `Remove ${marked.size} animation${marked.size === 1 ? '' : 's'}`}</button>
      </div>
    </Dialog>
  );
}
