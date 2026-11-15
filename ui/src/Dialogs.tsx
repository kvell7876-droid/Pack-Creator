import { useEffect, useRef, useState } from 'preact/hooks';
import type { ComponentChildren } from 'preact';
import { api } from './api';
import { GirlWizard } from './GirlWizard';
import { FileIcon } from './icons';
import { messageOf, notify } from './notify';
import type { Choices, FileKind, ImportNote, ImportResult } from './types';

export function Dialog({ title, onClose, children }: { title: string; onClose: () => void; children: ComponentChildren }) {
  return (
    <div class="scrim" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }} onKeyDown={event => { if (event.key === 'Escape') onClose(); }}>
      <div class="dialog" role="dialog" aria-label={title}>
        <h3>{title}</h3>
        {children}
      </div>
    </div>
  );
}

/** Asks for one line of text. */
export function Prompt({ title, label, initial, confirm, onDone, onClose }: { title: string; label: string; initial?: string; confirm: string; onDone: (value: string) => void; onClose: () => void }) {
  const [value, setValue] = useState(initial ?? '');

  return (
    <Dialog title={title} onClose={onClose}>
      <div class="field">
        <label>{label}</label>
        <input class="input" autoFocus value={value} onInput={e => setValue((e.target as HTMLInputElement).value)} onKeyDown={e => { if (e.key === 'Enter' && value.trim()) onDone(value.trim()); }} />
      </div>
      <div class="foot">
        <button class="btn ghost" onClick={onClose}>Cancel</button>
        <button class="btn primary" disabled={!value.trim()} onClick={() => onDone(value.trim())}>{confirm}</button>
      </div>
    </Dialog>
  );
}

export function Confirm({ title, text, confirm, onDone, onClose }: { title: string; text: string; confirm: string; onDone: () => void; onClose: () => void }) {
  return (
    <Dialog title={title} onClose={onClose}>
      <p style="margin:0">{text}</p>
      <div class="foot">
        <button class="btn ghost" onClick={onClose}>Cancel</button>
        <button class="btn danger" onClick={onDone}>{confirm}</button>
      </div>
    </Dialog>
  );
}

const KINDS: { kind: FileKind; label: string }[] = [
  { kind: 'girl', label: 'Girl' },
  { kind: 'scene', label: 'Scene' },
  { kind: 'trait', label: 'Trait' },
  { kind: 'dialogue', label: 'Dialogue' },
  { kind: 'social', label: 'Small talk' }
];

/** Makes a girl, scene, trait or dialogue file from a template. */
export function NewFileDialog({ initialKind, onCreated, onClose }: { initialKind: FileKind; onCreated: (rel: string) => void; onClose: () => void }) {
  const [kind, setKind] = useState<FileKind>(KINDS.some(k => k.kind === initialKind) ? initialKind : 'girl');
  const [choices, setChoices] = useState<Choices | null>(null);
  const [id, setId] = useState('');
  const [option, setOption] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void api.choices().then(setChoices);
  }, []);

  const idBad = id !== '' && !/^[a-z0-9_]+$/.test(id);
  const ready = id !== '' && !idBad;

  const create = async () => {
    setBusy(true);

    try {
      const optionValue = option || (kind === 'scene' ? choices?.scene_types[0] : kind === 'trait' ? choices?.trait_categories[0] : undefined);

      onCreated(await api.newFile({ kind, id, name: '', archetype: 'settler', body_type: '', tame_item: '', option: optionValue }));
    } catch (error) {
      notify(messageOf(error));
      setBusy(false);
    }
  };

  return (
    <Dialog title="New" onClose={onClose}>
      <div class="kinds">
        {KINDS.map(k => (
          <div key={k.kind} class={'kind' + (kind === k.kind ? ' on' : '')} onClick={() => setKind(k.kind)}>
            <FileIcon kind={k.kind} /> <div>{k.label}</div>
          </div>
        ))}
      </div>

      {kind === 'girl' ? (
        <GirlWizard choices={choices} onCreated={onCreated} />
      ) : (
        <>
      <div class="field">
        <label>Id (the file's name)</label>
        <input class={'input' + (idBad ? ' bad' : '')} autoFocus value={id} placeholder="amber" onInput={e => setId((e.target as HTMLInputElement).value)} />
        {idBad ? <span class="problem">Use lower case letters, digits and underscores only.</span> : <span class="hint">Lower case letters, digits and underscores.</span>}
      </div>

      {kind === 'scene' && (
        <div class="field">
          <label>Type</label>
          <select class="input" value={option} onChange={e => setOption((e.target as HTMLSelectElement).value)}>
            {(choices?.scene_types ?? []).map(t => <option key={t} value={t}>{t}</option>)}
          </select>
        </div>
      )}

      {kind === 'trait' && (
        <div class="field">
          <label>Category</label>
          <select class="input" value={option} onChange={e => setOption((e.target as HTMLSelectElement).value)}>
            {(choices?.trait_categories ?? []).map(t => <option key={t} value={t}>{t}</option>)}
          </select>
        </div>
      )}

      {kind === 'social' && (
        <div class="field">
          <label>Kind</label>
          <select class="input" value={option || 'line'} onChange={e => setOption((e.target as HTMLSelectElement).value)}>
            <option value="line">A line between two settlers</option>
            <option value="offer">An offer to the player</option>
          </select>
        </div>
      )}

      {kind === 'dialogue' && (
        <div class="field">
          <label>Topic</label>
          <input class="input" value={option} placeholder="chat" onInput={e => setOption((e.target as HTMLInputElement).value)} />
        </div>
      )}

      <div class="foot">
        <button class="btn ghost" onClick={onClose}>Cancel</button>
        <button class="btn primary" disabled={!ready || busy} onClick={create}>Create</button>
      </div>
        </>
      )}
    </Dialog>
  );
}

/** Opens any file of the pack by typing part of its name. */
export function QuickOpen({ onOpen, onClose }: { onOpen: (rel: string) => void; onClose: () => void }) {
  const [files, setFiles] = useState<string[]>([]);
  const [query, setQuery] = useState('');
  const [index, setIndex] = useState(0);
  const list = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void api.listFiles().then(setFiles);
  }, []);

  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  const shown = files.filter(f => words.every(w => f.toLowerCase().includes(w))).slice(0, 80);

  useEffect(() => { setIndex(0); }, [query]);

  useEffect(() => {
    (list.current?.children[index] as HTMLElement | undefined)?.scrollIntoView({ block: 'nearest' });
  }, [index]);

  const key = (event: KeyboardEvent) => {
    if (event.key === 'Escape') onClose();
    else if (event.key === 'ArrowDown') { event.preventDefault(); setIndex(i => Math.min(shown.length - 1, i + 1)); }
    else if (event.key === 'ArrowUp') { event.preventDefault(); setIndex(i => Math.max(0, i - 1)); }
    else if (event.key === 'Enter' && shown[index]) onOpen(shown[index]);
  };

  return (
    <div class="scrim" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }}>
      <div class="quick">
        <input autoFocus placeholder="Open a file by name…" value={query} onInput={e => setQuery((e.target as HTMLInputElement).value)} onKeyDown={key} />
        <div class="items" ref={list}>
          {shown.map((rel, i) => {
            const slash = rel.lastIndexOf('/');

            return (
              <div key={rel} class={'item' + (i === index ? ' on' : '')} onClick={() => onOpen(rel)}>
                <span>{rel.slice(slash + 1)}</span>
                <span class="dir">{slash >= 0 ? rel.slice(0, slash) : ''}</span>
              </div>
            );
          })}
          {shown.length === 0 && <div class="item muted">Nothing matches.</div>}
        </div>
      </div>
    </div>
  );
}

const LEVELS: Record<ImportNote['level'], string> = { error: 'Will not work until fixed', review: 'To look at', changed: 'Changed', dropped: 'Left behind', kept: 'Carried over' };

/** What an import did. */
export function ImportReport({ result, onOpen, onClose }: { result: ImportResult; onOpen: () => void; onClose: () => void }) {
  const [open, setOpen] = useState<Set<string>>(new Set());

  return (
    <Dialog title="Imported" onClose={onClose}>
      <div class="report">
        {(Object.keys(LEVELS) as ImportNote['level'][]).map(level => {
          const notes = result.notes.filter(n => n.level === level);

          return notes.length === 0 ? null : (
            <div key={level} class="report-group">
              <div class={'report-title ' + level}>{LEVELS[level]} ({notes.length})</div>
              {(open.has(level) ? notes : notes.slice(0, 6)).map((note, index) => <div key={index} class="report-line">{note.message}</div>)}
              {notes.length > 6 && !open.has(level) && <button class="btn ghost small" onClick={() => setOpen(previous => new Set(previous).add(level))}>Show {notes.length - 6} more</button>}
            </div>
          );
        })}
      </div>
      <div class="foot">
        <button class="btn ghost" onClick={onClose}>Close</button>
        <button class="btn primary" onClick={onOpen}>Open the girl</button>
      </div>
    </Dialog>
  );
}
