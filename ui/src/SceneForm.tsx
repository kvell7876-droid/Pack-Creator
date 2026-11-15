import { FieldInput, Labeled } from './GirlForm';
import type { Reference } from './types';

type Json = Record<string, unknown>;

/** A scene file as a form: one input for each setting the game has for a scene. It writes the whole file back, as the girl's form does. */
export function SceneForm({ text, reference, onChange, onShowJson }: { text: string; reference: Reference; onChange: (text: string) => void; onShowJson: () => void }) {
  let scene: Json | null = null;

  try {
    const parsed: unknown = JSON.parse(text);

    scene = typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed) ? (parsed as Json) : null;
  } catch {
    scene = null;
  }

  if (!scene) {
    return (
      <div class="form-pane">
        <div class="form-note">
          This file is not valid JSON right now, so the form cannot read it. Fix it in the JSON view (the problems panel says where), and the form comes back.
          <div style="margin-top:10px"><button class="btn" onClick={onShowJson}>Show the JSON</button></div>
        </div>
      </div>
    );
  }

  const current = scene;
  const set = (name: string, value: unknown) => {
    const next = { ...current };

    if (value === undefined) delete next[name];
    else next[name] = value;

    onChange(JSON.stringify(next, null, 2) + '\n');
  };

  /* The mini game has a section of its own below, and the overrides are edited in the JSON view. */
  const fields = reference.scene_fields.filter(field => field.name !== 'girl_overrides' && field.name !== 'minigame');

  return (
    <div class="form-pane">
      <div class="form-inner">
        <div class="form-card">
          <h3>Scene</h3>
          <div class="hint">Settings a scene sets for every girl who has it. Anything left empty uses the game's default.</div>
          <div class="form-grid">
            {fields.map(field => (
              <Labeled key={field.name} label={field.name.replace(/_/g, ' ')} hint={field.description}>
                <FieldInput field={field} value={current[field.name]} onChange={value => set(field.name, value)} />
              </Labeled>
            ))}
          </div>
          {'girl_overrides' in current && <div class="hint">This scene changes settings for some girls (girl_overrides). Edit those in the JSON view.</div>}
        </div>
        <MinigameCard value={current.minigame} onChange={value => set('minigame', value)} />
      </div>
    </div>
  );
}

/** The numbers of the pace game a scene may set, as the `minigame` block writes them. A band is a pair, [low, high]. */
const PACE_NUMBERS: { key: string; label: string; hint: string; band?: boolean; min: number; max: number; step: number }[] = [
  { key: 'slow_band', label: 'slow band', band: true, min: 0, max: 1, step: 0.01, hint: 'The pace she likes while she wants it slow, from 0 to 1.' },
  { key: 'fast_band', label: 'fast band', band: true, min: 0, max: 1, step: 0.01, hint: 'The pace she likes while she wants it fast, from 0 to 1.' },
  { key: 'slow_seconds', label: 'slow seconds', min: 1, max: 300, step: 1, hint: 'How long she wants it slow before the change.' },
  { key: 'fast_seconds', label: 'fast seconds', min: 1, max: 300, step: 1, hint: 'How long she wants it fast.' },
  { key: 'blend_seconds', label: 'blend seconds', min: 0.5, max: 30, step: 0.5, hint: 'How long the change between the two takes.' },
  { key: 'fill_seconds', label: 'fill seconds', min: 10, max: 600, step: 1, hint: 'How long her bar takes to fill at a good pace.' },
];

/**
 * The pace game for one scene. "Pace" is the game as set in the game's config with the feel read from the scene's clips; "Classic" keeps the old hold-the-key bar for this scene;
 * and any number set here replaces what is read from the clips. Written as the `minigame` field: a word for classic, or a block of only the numbers that are set.
 */
function MinigameCard({ value, onChange }: { value: unknown; onChange: (value: unknown) => void }) {
  const classic = value === 'classic' || (typeof value === 'object' && value !== null && (value as Json).classic === true);
  const block: Json = typeof value === 'object' && value !== null && !Array.isArray(value) ? (value as Json) : {};

  const setNumber = (key: string, next: unknown) => {
    const updated = { ...block };

    if (next === undefined) delete updated[key];
    else updated[key] = next;

    onChange(Object.keys(updated).length === 0 ? undefined : updated);
  };

  return (
    <div class="form-card">
      <h3>Mini game</h3>
      <div class="hint">How this scene is played. Leave the numbers empty and the pace game reads its feel from the scene's clips.</div>
      <div class="form-grid">
        <Labeled label="game" hint="Pace is the new game; classic is hold the key until the bar is full.">
          <select value={classic ? 'classic' : 'pace'} onChange={event => onChange((event.target as HTMLSelectElement).value === 'classic' ? 'classic' : undefined)}>
            <option value="pace">Pace (the game's setting)</option>
            <option value="classic">Classic for this scene</option>
          </select>
        </Labeled>
        {!classic && PACE_NUMBERS.map(item => {
          const current = block[item.key];
          const pair = Array.isArray(current) ? (current as number[]) : [undefined, undefined];

          if (item.band) {
            const setBand = (index: number, text: string) => {
              const next = [pair[0], pair[1]] as (number | undefined)[];

              next[index] = text === '' ? undefined : Number(text);
              setNumber(item.key, next[0] === undefined || next[1] === undefined ? undefined : next);
            };

            return (
              <Labeled key={item.key} label={item.label} hint={item.hint + ' Set both numbers, or neither.'}>
                <div style="display:flex;gap:6px">
                  <input type="number" min={item.min} max={item.max} step={item.step} value={pair[0] ?? ''} onChange={event => setBand(0, (event.target as HTMLInputElement).value)} />
                  <input type="number" min={item.min} max={item.max} step={item.step} value={pair[1] ?? ''} onChange={event => setBand(1, (event.target as HTMLInputElement).value)} />
                </div>
              </Labeled>
            );
          }

          return (
            <Labeled key={item.key} label={item.label} hint={item.hint}>
              <input
                type="number"
                min={item.min}
                max={item.max}
                step={item.step}
                value={typeof current === 'number' ? current : ''}
                onChange={event => {
                  const text = (event.target as HTMLInputElement).value;

                  setNumber(item.key, text === '' ? undefined : Number(text));
                }}
              />
            </Labeled>
          );
        })}
      </div>
    </div>
  );
}
