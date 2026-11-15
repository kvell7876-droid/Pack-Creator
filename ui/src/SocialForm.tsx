import { useState } from 'preact/hooks';
import { Labeled } from './GirlForm';
import type { Reference } from './types';

type Json = Record<string, unknown>;

const isObject = (value: unknown): value is Json => typeof value === 'object' && value !== null && !Array.isArray(value);

/* The words the game accepts in a small talk file's conditions (the game's SocialRegistry). Kept in step with `pc_core/src/social.rs`. */
const OUTCOMES = ['warm', 'easy', 'flat', 'clash'];
const PERSONALITIES = ['shy', 'bold', 'motherly', 'bratty', 'stoic'];
const JOBS = ['farmer', 'lumberjack', 'fisherman', 'guard', 'hunter', 'scout', 'alchemist', 'hauler', 'mineworker', 'blacksmith', 'courtesan'];
const MOODS = ['happy', 'content', 'tired', 'upset', 'afraid'];
const TIMES = ['day', 'night'];
const WEATHERS = ['clear', 'rain', 'thunder'];
const ROLES = ['starter', 'answerer'];

interface Props {
  text: string;
  reference: Reference;
  onChange: (text: string) => void;
  onShowJson: () => void;
}

/** A condition that takes one or several words. The file may write either; the form writes a list. */
function Choose({ value, options, onChange }: { value: unknown; options: string[]; onChange: (next: string[] | undefined) => void }) {
  const chosen = Array.isArray(value) ? value.map(String) : typeof value === 'string' ? [value] : [];

  return (
    <div class="tagrow">
      {options.map(option => (
        <button
          key={option}
          class={'tag' + (chosen.includes(option) ? ' on' : '')}
          onClick={() => { const next = chosen.includes(option) ? chosen.filter(o => o !== option) : [...chosen, option]; onChange(next.length ? next : undefined); }}
        >{option}</button>
      ))}
    </div>
  );
}

/** What a line needs, in words, for the entry's header: "warm or easy, farmer, rain". */
function summary(entry: Json): string {
  const parts: string[] = [];
  const words = (value: unknown) => (Array.isArray(value) ? value.join(' or ') : typeof value === 'string' ? value : '');

  for (const key of ['outcome', 'personality', 'job', 'mood']) {
    const text = words(entry[key]);

    if (text) parts.push(text);
  }

  for (const key of ['time', 'weather', 'interest', 'role']) {
    if (typeof entry[key] === 'string' && entry[key]) parts.push(String(entry[key]));
  }

  return parts.join(', ');
}

/**
 * A small talk file as a form: what settlers say on a break, one entry under another. Each has what is said, whether it is said between two settlers or offered to the player, and
 * the conditions it needs (how the chat is going, who is speaking, the time and the weather, an interest the pair share). The more conditions a line has the likelier it is, so the
 * form shows them in the header.
 */
export function SocialForm({ text, reference, onChange, onShowJson }: Props) {
  const [open, setOpen] = useState<number | null>(0);
  let entries: unknown[] | null = null;

  try {
    const parsed: unknown = JSON.parse(text);

    entries = Array.isArray(parsed) ? parsed : isObject(parsed) ? [parsed] : null;
  } catch {
    entries = null;
  }

  if (!entries) {
    return (
      <div class="form-pane">
        <div class="form-note">
          This file is not valid JSON right now, or is not a list of entries, so the form cannot read it. Fix it in the JSON view (the problems panel says where), and the form comes back.
          <div style="margin-top:10px"><button class="btn" onClick={onShowJson}>Show the JSON</button></div>
        </div>
      </div>
    );
  }

  const all = entries.filter(isObject);
  const write = (next: Json[]) => onChange(JSON.stringify(next, null, 2) + '\n');
  const ids = all.map(e => String(e.id ?? '')).filter(Boolean);

  const update = (index: number, key: string, value: unknown) => {
    write(all.map((entry, i) => {
      if (i !== index) return entry;

      const next = { ...entry };

      if (value === undefined || value === '') delete next[key];
      else next[key] = value;

      return next;
    }));
  };

  const add = (kind: 'line' | 'offer') => {
    let n = ids.length + 1;

    while (ids.includes(`${kind}_${n}`)) n++;

    write([...all, { id: `${kind}_${n}`, kind, text: '' }]);
    setOpen(all.length);
  };

  const move = (index: number, by: number) => {
    const target = index + by;

    if (target < 0 || target >= all.length) return;

    const next = [...all];

    [next[index], next[target]] = [next[target], next[index]];
    write(next);
    setOpen(target);
  };

  const duplicate = (index: number) => {
    let id = `${String(all[index].id ?? 'line')}_copy`;

    while (ids.includes(id)) id += '_2';

    write([...all.slice(0, index + 1), { ...all[index], id }, ...all.slice(index + 1)]);
    setOpen(index + 1);
  };

  return (
    <div class="form-pane">
      <div class="form-inner">
        <div class="form-head" style="padding:0 2px">
          <span class="muted">{all.length} entr{all.length === 1 ? 'y' : 'ies'}</span>
          <span style="margin-left:auto; display:flex; gap:6px">
            <button class="btn small" onClick={() => add('offer')}>＋ Offer to the player</button>
            <button class="btn small primary" onClick={() => add('line')}>＋ Line between settlers</button>
          </span>
        </div>

        {all.map((entry, index) => {
          const isOpen = open === index;
          const offer = entry.kind === 'offer';

          return (
            <div class="form-card" key={index}>
              <div class="form-head line-head" onClick={() => setOpen(isOpen ? null : index)}>
                <h3>{String(entry.id ?? '(no id)')}</h3>
                <span class="chip">{offer ? 'offer' : 'line'}</span>
                {summary(entry) && <span class="chip">{summary(entry)}</span>}
                <span class="muted line-preview">{String(entry.text ?? '').slice(0, 70)}</span>
                <span class="line-tools" onClick={e => e.stopPropagation()}>
                  <button class="btn ghost small" title="Move up" onClick={() => move(index, -1)}>↑</button>
                  <button class="btn ghost small" title="Move down" onClick={() => move(index, 1)}>↓</button>
                  <button class="btn ghost small" title="Copy this entry" onClick={() => duplicate(index)}>Copy</button>
                  <button class="btn ghost small" title="Remove this entry" onClick={() => write(all.filter((_, i) => i !== index))}>Remove</button>
                </span>
              </div>

              {isOpen && (
                <div style="margin-top:12px">
                  <div class="form-grid">
                    <Labeled label="Id" hint="A name for it: lower case letters, digits and _.">
                      <input class="input" value={String(entry.id ?? '')} onInput={e => update(index, 'id', (e.target as HTMLInputElement).value)} />
                    </Labeled>
                    <Labeled label="Kind" hint="A line is said between two settlers. An offer is said to the player when a settler on break walks over to them.">
                      <select class="input" value={offer ? 'offer' : 'line'} onChange={e => update(index, 'kind', (e.target as HTMLSelectElement).value)}>
                        <option value="line">Line between settlers</option>
                        <option value="offer">Offer to the player</option>
                      </select>
                    </Labeled>
                  </div>

                  <Labeled label="What is said" required hint="{name} is the speaker, {partner} the other settler (or the player for an offer), {interest} what they share.">
                    <textarea class="input area" rows={2} value={String(entry.text ?? '')} onInput={e => update(index, 'text', (e.target as HTMLTextAreaElement).value)} />
                  </Labeled>

                  <details class="more" open>
                    <summary>When it fits (leave a group empty to match anything)</summary>

                    {!offer && (
                      <Labeled label="How the chat is going">
                        <Choose value={entry.outcome} options={OUTCOMES} onChange={next => update(index, 'outcome', next)} />
                      </Labeled>
                    )}
                    <Labeled label="The speaker's personality">
                      <Choose value={entry.personality} options={PERSONALITIES} onChange={next => update(index, 'personality', next)} />
                    </Labeled>
                    <Labeled label="The speaker's job">
                      <Choose value={entry.job} options={JOBS} onChange={next => update(index, 'job', next)} />
                    </Labeled>
                    <Labeled label="The speaker's mood">
                      <Choose value={entry.mood} options={MOODS} onChange={next => update(index, 'mood', next)} />
                    </Labeled>

                    <div class="form-grid">
                      <Labeled label="Time of day">
                        <select class="input" value={String(entry.time ?? '')} onChange={e => update(index, 'time', (e.target as HTMLSelectElement).value || undefined)}>
                          <option value="">(any)</option>
                          {TIMES.map(t => <option key={t} value={t}>{t}</option>)}
                        </select>
                      </Labeled>
                      <Labeled label="Weather">
                        <select class="input" value={String(entry.weather ?? '')} onChange={e => update(index, 'weather', (e.target as HTMLSelectElement).value || undefined)}>
                          <option value="">(any)</option>
                          {WEATHERS.map(t => <option key={t} value={t}>{t}</option>)}
                        </select>
                      </Labeled>
                      {!offer && (
                        <>
                          <Labeled label="A shared interest" hint="Only when the pair both like it. Hearing it teaches the player they do.">
                            <select class="input" value={String(entry.interest ?? '')} onChange={e => update(index, 'interest', (e.target as HTMLSelectElement).value || undefined)}>
                              <option value="">(any)</option>
                              {reference.taste_tags.map(t => <option key={t} value={t}>{t}</option>)}
                            </select>
                          </Labeled>
                          <Labeled label="Who says it" hint="The one who began the chat, or the one who answers.">
                            <select class="input" value={String(entry.role ?? '')} onChange={e => update(index, 'role', (e.target as HTMLSelectElement).value || undefined)}>
                              <option value="">(either)</option>
                              {ROLES.map(t => <option key={t} value={t}>{t}</option>)}
                            </select>
                          </Labeled>
                        </>
                      )}
                      <Labeled label="How often" hint="Weight: higher is picked more. The more conditions a line has the likelier it is already.">
                        <input class="input" inputMode="numeric" value={String(entry.weight ?? '')} placeholder="1" onInput={e => { const raw = (e.target as HTMLInputElement).value.trim(); update(index, 'weight', raw === '' ? undefined : Number.isFinite(Number(raw)) ? Math.max(1, Math.round(Number(raw))) : entry.weight); }} />
                      </Labeled>
                    </div>
                  </details>
                </div>
              )}
            </div>
          );
        })}

        {all.length === 0 && <div class="empty">Nothing here yet. Add a line between settlers, or an offer to the player.</div>}
      </div>
    </div>
  );
}
