import { useEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api';
import { messageOf, notify } from '../notify';
import type { ClipData, CueEvent } from '../types';

const KINDS = ['instruction', 'sound', 'particle'];
const AUTHORITIES = [
  { id: 0, label: 'Event type decides' },
  { id: 1, label: 'Server' },
  { id: 2, label: 'Client' },
  { id: 3, label: 'Both' }
];

interface Props {
  /** The animation file and the clip being edited. */
  rel: string;
  clip: ClipData;
  /** Where the playhead is, for "add at the playhead". */
  time: number;
  onSeek: (time: number) => void;
  /** The clips as the file holds them after a save. */
  onSaved: (clips: ClipData[]) => void;
}

const same = (a: CueEvent[], b: CueEvent[]) => a.length === b.length && a.every((e, i) => e.time === b[i].time && e.kind === b[i].kind && e.payload === b[i].payload && e.authority === b[i].authority);

/**
 * The cues of one clip as a list that can be edited: when each fires, what kind it is and what it says. An instruction is one or more cue names joined with dashes
 * (`pound-doggymoan`); a name nothing in the game or the pack fires is flagged, because it would never do anything. Edits save themselves a moment after the last one;
 * only the cues change, the motion in the file is copied as it was.
 */
export function CueEditor({ rel, clip, time, onSeek, onSaved }: Props) {
  const [draft, setDraft] = useState<CueEvent[]>(clip.events);
  const [names, setNames] = useState<Set<string>>(new Set());
  const [state, setState] = useState<'saved' | 'waiting' | 'failed'>('saved');
  const timer = useRef<number | undefined>(undefined);
  const saved = useRef<CueEvent[]>(clip.events);

  /* Another clip, or the file reloaded: start again from what the file holds. */
  useEffect(() => {
    setDraft(clip.events);
    saved.current = clip.events;
    setState('saved');
  }, [rel, clip.name]);

  useEffect(() => { void api.cueNames().then(list => setNames(new Set(list))).catch(() => undefined); }, [rel]);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  const save = async (events: CueEvent[]) => {
    try {
      const clips = await api.saveClipEvents(rel, clip.name, events);

      saved.current = events;
      setState('saved');
      onSaved(clips);
    } catch (error) {
      setState('failed');
      notify(messageOf(error));
    }
  };

  const change = (next: CueEvent[]) => {
    setDraft(next);
    window.clearTimeout(timer.current);

    if (same(next, saved.current)) {
      setState('saved');
      return;
    }

    setState('waiting');
    timer.current = window.setTimeout(() => void save(next), 700);
  };

  const patch = (index: number, change_: Partial<CueEvent>) => change(draft.map((event, i) => (i === index ? { ...event, ...change_ } : event)));

  const add = () => {
    const at = Math.max(0, Math.min(Math.round(time * 100) / 100, clip.duration));

    change([...draft, { time: at, kind: 'instruction', payload: '', authority: 0 }]);
  };

  /* A cue name that is not one the game or this pack fires. Sound and particle payloads are ids, not cue names. */
  const unknown = (event: CueEvent) => (event.kind !== 'instruction' || names.size === 0 ? [] : event.payload.toLowerCase().split('-').filter(part => part && !names.has(part)));

  return (
    <div class="cue-editor">
      <datalist id="cue-names">{[...names].map(name => <option key={name} value={name} />)}</datalist>

      <div class="cue-editor-head">
        <b>Cues of {clip.name.replace(/^animation\./, '')}</b>
        <span class="muted">{state === 'saved' ? 'saved' : state === 'waiting' ? 'saving...' : 'could not save'}</span>
        <button class="btn small primary" style="margin-left:auto" onClick={add} title="Add a cue at the time shown on the slider">＋ Add a cue at {time.toFixed(2)} s</button>
      </div>

      {draft.length === 0 && <div class="muted" style="padding:6px 2px">This clip fires no cues. Scrub to the moment and add one.</div>}

      {draft.map((event, index) => {
        const bad = unknown(event);

        return (
          <div class="cue-row" key={index}>
            <input class="input cue-time" type="number" step="0.01" min="0" max={clip.duration} value={event.time} title="When it fires, in seconds"
              onInput={e => { const n = Number((e.target as HTMLInputElement).value); if (Number.isFinite(n)) patch(index, { time: Math.max(0, Math.min(n, clip.duration)) }); }} />
            <button class="btn ghost small" title="Show the model at this moment" onClick={() => onSeek(event.time)}>Go</button>
            <select class="input small-select" value={event.kind} onChange={e => patch(index, { kind: (e.target as HTMLSelectElement).value })}>
              {(KINDS.includes(event.kind) ? KINDS : [...KINDS, event.kind]).map(kind => <option key={kind} value={kind}>{kind}</option>)}
            </select>
            <input class={'input cue-payload' + (bad.length ? ' bad' : '')} list={event.kind === 'instruction' ? 'cue-names' : undefined} value={event.payload}
              placeholder={event.kind === 'instruction' ? 'pound-moan (cue names joined with dashes)' : event.kind === 'sound' ? 'pleasurehorizons:misc.pounding' : 'effect'}
              onInput={e => patch(index, { payload: (e.target as HTMLInputElement).value })} />
            <select class="input small-select" value={event.authority} title="Who decides that it fires" onChange={e => patch(index, { authority: Number((e.target as HTMLSelectElement).value) })}>
              {(AUTHORITIES.some(a => a.id === event.authority) ? AUTHORITIES : [...AUTHORITIES, { id: event.authority, label: `Authority ${event.authority}` }]).map(a => <option key={a.id} value={a.id}>{a.label}</option>)}
            </select>
            <button class="btn ghost small" title="Remove this cue" onClick={() => change(draft.filter((_, i) => i !== index))}>Remove</button>
            {bad.length > 0 && <div class="cue-warn">{bad.map(b => `"${b}"`).join(', ')} is not a cue the game or this pack fires anywhere else. Check the spelling; if it is new, bind it to a sound in an event bindings file.</div>}
          </div>
        );
      })}
    </div>
  );
}
