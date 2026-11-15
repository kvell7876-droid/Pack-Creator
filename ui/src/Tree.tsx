import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from './api';
import { Chevron, FileIcon, FolderIcon } from './icons';
import { messageOf, notify } from './notify';
import { hasFiles } from './dropped';
import type { Diagnostic, Entry } from './types';

interface Props {
  /** Changes whenever files were made, renamed or deleted, so open folders are read again. */
  version: number;
  active: string | null;
  /** What was last clicked in the tree, a file or a folder: new files and folders are made under it. */
  selected: string | null;
  /** Changes when the person asks to fold every folder. */
  collapseKey?: number;
  problems: Record<string, Diagnostic[]>;
  onOpen: (rel: string) => void;
  onSelect: (entry: Entry | null) => void;
  /** An entry was dropped on a folder ('' is the top of the pack). */
  onMove: (entry: Entry, folder: string) => void;
  /** Files were dropped from another program onto a folder ('' is the top of the pack). The transfer has to be read straight away, in this call. */
  onDropFiles: (transfer: DataTransfer, folder: string) => void;
  onContext: (entry: Entry | null, x: number, y: number) => void;
}

/** How many errors and warnings each file and each folder holds, worked out once for every file with problems (not once for every row). */
function tally(problems: Record<string, Diagnostic[]>): Map<string, { error: number; warning: number }> {
  const totals = new Map<string, { error: number; warning: number }>();

  for (const [file, list] of Object.entries(problems)) {
    let error = 0;
    let warning = 0;

    for (const d of list) {
      if (d.severity === 'error') error++;
      else if (d.severity === 'warning') warning++;
    }

    if (error === 0 && warning === 0) continue;

    /* The file itself and every folder above it. */
    for (let at = file.length; at > 0; at = file.lastIndexOf('/', at - 1)) {
      const key = file.slice(0, at);
      const entry = totals.get(key) ?? { error: 0, warning: 0 };

      entry.error += error;
      entry.warning += warning;
      totals.set(key, entry);

      if (at === -1) break;
    }
  }

  return totals;
}

/** The bytes of a file, ready as an address the browser can fetch, so that dragging the file out of the window (onto the desktop, a folder, another program) makes a copy of it there. */
const outgoing = new Map<string, string>();

/** The most a file may be to be offered for dragging out, in bytes: it is read whole before the drag, and has to be ready when the drag begins. */
const DRAG_OUT_LIMIT = 25 * 1024 * 1024;

const MIME: Record<string, string> = { json: 'application/json', png: 'image/png', jpg: 'image/jpeg', ogg: 'audio/ogg', mcmeta: 'application/json', txt: 'text/plain' };

/** Reads a file at the moment the pointer goes down on it, which is a moment before a drag can start. */
async function prepareDragOut(entry: Entry) {
  if (entry.is_dir || entry.size > DRAG_OUT_LIMIT || outgoing.has(entry.rel)) return;

  try {
    const base64 = await api.readAsset(entry.rel);
    const bytes = Uint8Array.from(atob(base64), c => c.charCodeAt(0));
    const extension = entry.name.includes('.') ? entry.name.slice(entry.name.lastIndexOf('.') + 1).toLowerCase() : '';

    outgoing.set(entry.rel, URL.createObjectURL(new Blob([bytes], { type: MIME[extension] ?? 'application/octet-stream' })));
    window.setTimeout(() => { const url = outgoing.get(entry.rel); if (url) URL.revokeObjectURL(url); outgoing.delete(entry.rel); }, 60000);
  } catch {
    /* It is only not offered for dragging out. */
  }
}

/* The entry being dragged. A drag carries only text, and the page knows which entry it was. */
let dragged: Entry | null = null;

/** Whether `entry` may be dropped into `folder`: not where it already is, and a folder not into itself or something inside itself. */
function canDrop(entry: Entry | null, folder: string): boolean {
  if (!entry) return false;

  const parent = entry.rel.includes('/') ? entry.rel.slice(0, entry.rel.lastIndexOf('/')) : '';

  if (parent === folder) return false;

  return !(entry.is_dir && (folder === entry.rel || folder.startsWith(entry.rel + '/')));
}

export function Tree({ version, active, selected, collapseKey, problems, onOpen, onSelect, onMove, onDropFiles, onContext }: Props) {
  const [dropOn, setDropOn] = useState<string | null>(null);
  const [children, setChildren] = useState<Record<string, Entry[]>>({});
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const totals = useMemo(() => tally(problems), [problems]);

  const load = async (rel: string): Promise<Entry[]> => {
    try {
      const entries = await api.listDir(rel);

      setChildren(previous => ({ ...previous, [rel]: entries }));
      return entries;
    } catch (error) {
      notify(messageOf(error));
      return [];
    }
  };

  /* The first time, the top of the pack is opened down to the namespace, which is where the work is. */
  useEffect(() => {
    void (async () => {
      const open = new Set<string>(['']);
      const top = await load('');

      if (top.some(e => e.rel === 'data')) {
        open.add('data');

        for (const ns of await load('data')) {
          if (ns.is_dir) {
            open.add(ns.rel);
            await load(ns.rel);
          }
        }
      }

      setExpanded(open);
    })();
  }, []);

  /* Fold every folder, leaving the top of the pack open. */
  useEffect(() => {
    if (collapseKey) setExpanded(new Set(['']));
  }, [collapseKey]);

  /* After a change, every open folder is read again. */
  useEffect(() => {
    if (version === 0) return;

    for (const rel of expanded) void load(rel);
  }, [version]);

  const toggle = async (entry: Entry) => {
    const next = new Set(expanded);

    if (next.has(entry.rel)) {
      next.delete(entry.rel);
    } else {
      next.add(entry.rel);
      await load(entry.rel);
    }

    setExpanded(next);
  };

  const row = (entry: Entry, depth: number) => {
    const open = expanded.has(entry.rel);
    const have = totals.get(entry.rel);
    const errors = have?.error ?? 0;
    const warnings = errors === 0 ? have?.warning ?? 0 : 0;

    return (
      <div key={entry.rel}>
        <div
          class={'tree-row' + (active === entry.rel ? ' active' : '') + (selected === entry.rel ? ' selected' : '') + (dropOn === entry.rel ? ' drop' : '')}
          style={{ paddingLeft: 8 + depth * 14 }}
          draggable
          onMouseDown={() => void prepareDragOut(entry)}
          onDragStart={event => {
            dragged = entry;
            event.dataTransfer?.setData('text/plain', entry.rel);

            /* A file that is ready is also offered to whatever it is dropped on outside the window, as a file with its own name. */
            const url = outgoing.get(entry.rel);
            const extension = entry.name.includes('.') ? entry.name.slice(entry.name.lastIndexOf('.') + 1).toLowerCase() : '';

            if (url) event.dataTransfer?.setData('DownloadURL', `${MIME[extension] ?? 'application/octet-stream'}:${entry.name}:${url}`);
            if (event.dataTransfer) event.dataTransfer.effectAllowed = url ? 'copyMove' : 'move';
          }}
          onDragEnd={() => { dragged = null; setDropOn(null); }}
          onDragOver={event => {
            /* A folder takes what is dropped on it; a file stands for the folder it is in. */
            const folder = entry.is_dir ? entry.rel : entry.rel.includes('/') ? entry.rel.slice(0, entry.rel.lastIndexOf('/')) : '';

            if (canDrop(dragged, folder) || (!dragged && hasFiles(event.dataTransfer))) { event.preventDefault(); event.stopPropagation(); setDropOn(entry.is_dir ? entry.rel : folder); }
          }}
          onDrop={event => {
            const folder = entry.is_dir ? entry.rel : entry.rel.includes('/') ? entry.rel.slice(0, entry.rel.lastIndexOf('/')) : '';

            event.preventDefault();
            event.stopPropagation();
            setDropOn(null);

            if (dragged && canDrop(dragged, folder)) {
              if (folder) setExpanded(previous => new Set(previous).add(folder));
              onMove(dragged, folder);
            } else if (!dragged && event.dataTransfer && hasFiles(event.dataTransfer)) {
              if (folder) setExpanded(previous => new Set(previous).add(folder));
              onDropFiles(event.dataTransfer, folder);
            }

            dragged = null;
          }}
          onClick={event => { event.stopPropagation(); onSelect(entry); if (entry.is_dir) void toggle(entry); else onOpen(entry.rel); }}
          onContextMenu={event => { event.preventDefault(); event.stopPropagation(); onContext(entry, event.clientX, event.clientY); }}
        >
          {entry.is_dir ? <Chevron open={open} /> : <span style={{ width: 14 }} />}
          {entry.is_dir ? <FolderIcon open={open} /> : <FileIcon kind={entry.kind} />}
          <span class="label">{entry.name}</span>
          {errors > 0 && <span class="badge error">{errors}</span>}
          {warnings > 0 && <span class="badge warning">{warnings}</span>}
        </div>
        {entry.is_dir && open && (children[entry.rel] ?? []).map(child => row(child, depth + 1))}
      </div>
    );
  };

  return (
    <div
      class={'tree' + (dropOn === '' ? ' drop' : '')}
      onClick={() => onSelect(null)}
      onDragOver={event => { if (canDrop(dragged, '') || (!dragged && hasFiles(event.dataTransfer))) { event.preventDefault(); setDropOn(''); } }}
      onDragLeave={event => { if (event.target === event.currentTarget) setDropOn(null); }}
      onDrop={event => {
        event.preventDefault();
        setDropOn(null);

        if (dragged && canDrop(dragged, '')) onMove(dragged, '');
        else if (!dragged && event.dataTransfer && hasFiles(event.dataTransfer)) onDropFiles(event.dataTransfer, '');

        dragged = null;
      }}
      onContextMenu={event => { event.preventDefault(); onContext(null, event.clientX, event.clientY); }}
    >
      {(children[''] ?? []).map(entry => row(entry, 0))}
    </div>
  );
}
