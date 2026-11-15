import { useState } from 'preact/hooks';
import { Labeled } from './GirlForm';
import type { Reference } from './types';

type Json = Record<string, unknown>;

const isObject = (value: unknown): value is Json => typeof value === 'object' && value !== null && !Array.isArray(value);

interface Props {
  text: string;
  reference: Reference;
  onChange: (text: string) => void;
  onShowJson: () => void;
}

function Tags({ value, options, onChange }: { value: unknown; options: string[]; onChange: (tags: string[] | undefined) => void }) {
  const chosen = Array.isArray(value) ? value.map(String) : [];

  return (
    <div class="tagrow">
      {options.map(tag => (
        <button
          key={tag}
          class={'tag' + (chosen.includes(tag) ? ' on' : '')}
          onClick={() => { const next = chosen.includes(tag) ? chosen.filter(t => t !== tag) : [...chosen, tag]; onChange(next.length ? next : undefined); }}
        >{tag}</button>
      ))}
    </div>
  );
}

/**
 * A dialogue file as a form: its lines one under another, each with what is said, how she replies, the taste tags it carries and, folded away, what it needs of the
 * moment. A line she opens (one with answers) lists the answers the player may give. Lines can be added, copied, moved and removed.
 */
export function DialogueForm({ text, reference, onChange, onShowJson }: Props) {
  const [open, setOpen] = useState<number | null>(0);
  let lines: unknown[] | null = null;

  try {
    const parsed: unknown = JSON.parse(text);

    lines = Array.isArray(parsed) ? parsed : null;
  } catch {
    lines = null;
  }

  if (!lines) {
    return (
      <div class="form-pane">
        <div class="form-note">
          This file is not valid JSON right now, or is not a list of lines, so the form cannot read it. Fix it in the JSON view (the problems panel says where), and the form comes back.
          <div style="margin-top:10px"><button class="btn" onClick={onShowJson}>Show the JSON</button></div>
        </div>
      </div>
    );
  }

  const all = lines.filter(isObject);
  const write = (next: Json[]) => onChange(JSON.stringify(next, null, 2) + '\n');
  const ownIds = all.map(l => String(l.id ?? '')).filter(Boolean);
  const sayable = [...new Set([...ownIds, ...reference.dialogue_ids])].sort();

  const update = (index: number, key: string, value: unknown) => {
    write(all.map((line, i) => {
      if (i !== index) return line;

      const next = { ...line };

      if (value === undefined || value === '') delete next[key];
      else next[key] = value;

      return next;
    }));
  };

  const add = () => {
    let n = ownIds.length + 1;

    while (ownIds.includes(`line_${n}`)) n++;

    write([...all, { id: `line_${n}`, topic: reference.dialogue_topics[0] ?? 'chat', tags: [], player_line: '', reply_good: '', reply_bad: '' }]);
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

    while (ownIds.includes(id)) id += '_2';

    write([...all.slice(0, index + 1), { ...all[index], id }, ...all.slice(index + 1)]);
    setOpen(index + 1);
  };

  const text3 = (index: number, key: string, label: string, rows = 2) => (
    <Labeled label={label}>
      <textarea class="input area" rows={rows} value={String(all[index][key] ?? '')} onInput={e => update(index, key, (e.target as HTMLTextAreaElement).value)} />
    </Labeled>
  );

  return (
    <div class="form-pane">
      <div class="form-inner">
        <div class="form-head" style="padding:0 2px"><span class="muted">{all.length} line{all.length === 1 ? '' : 's'}</span><button class="btn small primary" style="margin-left:auto" onClick={add}>＋ Add a line</button></div>

        {all.map((line, index) => {
          const isOpen = open === index;
          const answers = Array.isArray(line.answers) ? line.answers.filter(isObject) : [];

          return (
            <div class="form-card" key={index}>
              <div class="form-head line-head" onClick={() => setOpen(isOpen ? null : index)}>
                <h3>{String(line.id ?? '(no id)')}</h3>
                <span class="chip">{String(line.topic ?? '')}</span>
                {answers.length > 0 && <span class="chip">she asks</span>}
                <span class="muted line-preview">{String(line.player_line ?? '').slice(0, 70)}</span>
                <span class="line-tools" onClick={e => e.stopPropagation()}>
                  <button class="btn ghost small" title="Move up" onClick={() => move(index, -1)}>↑</button>
                  <button class="btn ghost small" title="Move down" onClick={() => move(index, 1)}>↓</button>
                  <button class="btn ghost small" title="Copy this line" onClick={() => duplicate(index)}>Copy</button>
                  <button class="btn ghost small" title="Remove this line" onClick={() => write(all.filter((_, i) => i !== index))}>Remove</button>
                </span>
              </div>

              {isOpen && (
                <div style="margin-top:12px">
                  <div class="form-grid">
                    <Labeled label="Id" required hint="Unique across dialogue: lower case letters, digits and _.">
                      <input class="input" value={String(line.id ?? '')} onInput={e => update(index, 'id', (e.target as HTMLInputElement).value)} />
                    </Labeled>
                    <Labeled label="Topic" required>
                      <select class="input" value={String(line.topic ?? '')} onChange={e => update(index, 'topic', (e.target as HTMLSelectElement).value)}>
                        {reference.dialogue_topics.map(t => <option key={t} value={t}>{t}</option>)}
                      </select>
                    </Labeled>
                  </div>

                  <Labeled label="Tags" hint="What the line appeals to. She likes or dislikes it by her own taste.">
                    <Tags value={line.tags} options={reference.taste_tags} onChange={tags => update(index, 'tags', tags)} />
                  </Labeled>

                  {text3(index, 'player_line', answers.length > 0 ? 'What she asks' : 'What is said', 2)}

                  {answers.length === 0 && (
                    <div class="form-grid">
                      {text3(index, 'reply_good', 'Her reply when it lands')}
                      {text3(index, 'reply_bad', 'Her reply when it does not')}
                    </div>
                  )}

                  <details class="more" open={answers.length > 0}>
                    <summary>Answers (makes it a line she opens)</summary>
                    {answers.map((answer, a) => {
                      const setAnswer = (key: string, value: unknown) => {
                        const next = answers.map((x, i) => {
                          if (i !== a) return x;

                          const copy = { ...x };

                          if (value === undefined || value === '') delete copy[key];
                          else copy[key] = value;

                          return copy;
                        });

                        update(index, 'answers', next);
                      };

                      return (
                        <div class="row-card" key={a}>
                          <div class="grow">
                            <Labeled label="The player's answer"><input class="input" value={String(answer.text ?? '')} onInput={e => setAnswer('text', (e.target as HTMLInputElement).value)} /></Labeled>
                            <Tags value={answer.tags} options={reference.taste_tags} onChange={tags => setAnswer('tags', tags)} />
                            <div class="form-grid" style="margin-top:8px">
                              <Labeled label="Her reply when it lands"><input class="input" value={String(answer.reply_good ?? '')} onInput={e => setAnswer('reply_good', (e.target as HTMLInputElement).value)} /></Labeled>
                              <Labeled label="Her reply when it does not"><input class="input" value={String(answer.reply_bad ?? '')} onInput={e => setAnswer('reply_bad', (e.target as HTMLInputElement).value)} /></Labeled>
                            </div>
                          </div>
                          <button class="btn ghost small" onClick={() => update(index, 'answers', answers.filter((_, i) => i !== a))}>Remove</button>
                        </div>
                      );
                    })}
                    <button class="btn small" onClick={() => update(index, 'answers', [...answers, { text: '', tags: [], reply_good: '', reply_bad: '' }])}>＋ Answer</button>
                  </details>

                  <details class="more">
                    <summary>When it is offered</summary>
                    <div class="form-grid">
                      <Labeled label="Needs affection" hint="0 to 10.">
                        <input class="input" inputMode="numeric" value={String(line.requires_affection ?? '')} onInput={e => { const raw = (e.target as HTMLInputElement).value.trim(); update(index, 'requires_affection', raw === '' ? undefined : Number.isFinite(Number(raw)) ? Math.round(Number(raw)) : line.requires_affection); }} />
                      </Labeled>
                      <Labeled label="Needs the conversation going" hint="How well it must be going.">
                        <input class="input" inputMode="numeric" value={String(line.requires_flow ?? '')} onInput={e => { const raw = (e.target as HTMLInputElement).value.trim(); update(index, 'requires_flow', raw === '' ? undefined : Number.isFinite(Number(raw)) ? Math.round(Number(raw)) : line.requires_flow); }} />
                      </Labeled>
                      <Labeled label="Follows the line" hint="Only after this line has been said.">
                        <select class="input" value={String(line.requires_said ?? '')} onChange={e => update(index, 'requires_said', (e.target as HTMLSelectElement).value || undefined)}>
                          <option value="">(any time)</option>
                          {sayable.filter(id => id !== line.id).map(id => <option key={id} value={id}>{id}</option>)}
                        </select>
                      </Labeled>
                      <Labeled label="How often" hint="Weight: higher is picked more.">
                        <input class="input" inputMode="numeric" value={String(line.weight ?? '')} placeholder="1" onInput={e => { const raw = (e.target as HTMLInputElement).value.trim(); update(index, 'weight', raw === '' ? undefined : Number.isFinite(Number(raw)) ? Math.max(1, Math.round(Number(raw))) : line.weight); }} />
                      </Labeled>
                      <Labeled label="Only once">
                        <label class="check-inline"><input type="checkbox" checked={line.once === true} onChange={e => update(index, 'once', (e.target as HTMLInputElement).checked || undefined)} /> never offered again once said</label>
                      </Labeled>
                      <Labeled label="Not for these girls" hint="Girl ids, separated by commas.">
                        <input class="input" value={Array.isArray(line.unsupported_girls) ? line.unsupported_girls.join(', ') : ''} onInput={e => { const list = (e.target as HTMLInputElement).value.split(',').map(v => v.trim()).filter(Boolean); update(index, 'unsupported_girls', list.length ? list : undefined); }} />
                      </Labeled>
                    </div>
                  </details>
                </div>
              )}
            </div>
          );
        })}

        {all.length === 0 && <div class="empty">No lines yet. Press <b>Add a line</b>.</div>}
      </div>
    </div>
  );
}
