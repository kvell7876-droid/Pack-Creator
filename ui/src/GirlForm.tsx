import { useState } from 'preact/hooks';
import type { RComponent, RField, Reference } from './types';

/**
 * The guided way to write a girl: a form over the same file the JSON editor shows. It reads the file, changes one value, and writes the whole file back, so the form and
 * the text never disagree. What the form offers comes from the game's reference, so a setting the game gains appears with no change here.
 */

type Json = Record<string, unknown>;

const ORDER = ['identity', 'body', 'taste', 'stats', 'personality', 'dialogue', 'ai', 'spawn', 'merchant'];
/** Components that only one kind of girl has. */
const ONLY_FOR: Record<string, string> = { spawn: 'wild', merchant: 'merchant' };

const short = (id: string) => id.slice(id.indexOf(':') + 1);
const isObject = (value: unknown): value is Json => typeof value === 'object' && value !== null && !Array.isArray(value);

interface Props {
  text: string;
  reference: Reference;
  packTraits: string[];
  onChange: (text: string) => void;
  onShowJson: () => void;
}

export function GirlForm({ text, reference, packTraits, onChange, onShowJson }: Props) {
  let girl: Json | null = null;

  try {
    const parsed: unknown = JSON.parse(text);

    girl = isObject(parsed) ? parsed : null;
  } catch {
    girl = null;
  }

  if (!girl) {
    return (
      <div class="form-pane">
        <div class="form-note">
          This file is not valid JSON right now, so the form cannot read it. Fix it in the JSON view (the problems panel says where), and the form comes back.
          <div style="margin-top:10px"><button class="btn" onClick={onShowJson}>Show the JSON</button></div>
        </div>
      </div>
    );
  }

  const root = girl;
  const archetype = typeof root.type === 'string' ? short(root.type) : 'settler';
  const components: Json = isObject(root.components) ? root.components : {};

  const write = (next: Json) => onChange(JSON.stringify(next, null, 2) + '\n');
  const setRoot = (key: string, value: unknown) => write({ ...root, [key]: value });
  const setComponent = (id: string, value: Json | undefined) => {
    const next = { ...components };

    if (value === undefined) delete next[id];
    else next[id] = value;

    write({ ...root, components: next });
  };

  const entries = Object.values(reference.components)
    .filter(component => !ONLY_FOR[short(component.id)] || ONLY_FOR[short(component.id)] === archetype)
    .sort((a, b) => ORDER.indexOf(short(a.id)) - ORDER.indexOf(short(b.id)));
  const needed = reference.archetypes['pleasurehorizons:' + archetype] ?? [];

  /** A component is written with its full id; one written without a namespace is found too. */
  const keyOf = (component: RComponent): string | undefined => Object.keys(components).find(key => key === component.id || key === short(component.id));

  return (
    <div class="form-pane">
      <div class="form-inner">
        <div class="form-card">
          <h3>Her</h3>
          <div class="form-grid">
            <Labeled label="Id" hint="The file's name, in lower case letters, digits and underscores.">
              <input class="input" value={String(root.id ?? '')} onInput={event => setRoot('id', (event.target as HTMLInputElement).value)} />
            </Labeled>
            <Labeled label="Kind of girl" hint="A settler lives in a settlement, a wild girl is met out in the world, a merchant travels and trades.">
              <select class="input" value={archetype} onChange={event => setRoot('type', (event.target as HTMLSelectElement).value)}>
                {Object.keys(reference.archetypes).map(id => <option key={id} value={short(id)}>{short(id)}</option>)}
              </select>
            </Labeled>
          </div>
        </div>

        {entries.map(component => {
          const key = keyOf(component);
          const present = key !== undefined;
          const required = needed.includes(component.id);
          const body: Json = present && isObject(components[key]) ? (components[key] as Json) : {};

          return (
            <div class="form-card" key={component.id}>
              <div class="form-head">
                <h3>{short(component.id)}</h3>
                {required
                  ? <span class="chip">needed</span>
                  : <label class="check-inline"><input type="checkbox" checked={present} onChange={event => setComponent(key ?? component.id, (event.target as HTMLInputElement).checked ? {} : undefined)} /> include</label>}
              </div>
              <div class="hint">{component.description}</div>

              {(present || required) && (short(component.id) === 'ai'
                ? <Traits body={body} reference={reference} packTraits={packTraits} archetype={archetype} onChange={value => setComponent(key ?? component.id, value)} />
                : (
                  <div class="form-grid">
                    {component.fields.map(field => (
                      <Labeled key={field.name} label={field.name.replace(/_/g, ' ')} required={field.required} hint={field.description}>
                        <FieldInput
                          field={field}
                          value={body[field.name]}
                          onChange={value => {
                            const next = { ...body };

                            if (value === undefined) delete next[field.name];
                            else next[field.name] = value;

                            setComponent(key ?? component.id, next);
                          }}
                        />
                      </Labeled>
                    ))}
                  </div>
                ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function Labeled({ label, hint, required, children }: { label: string; hint?: string; required?: boolean; children: preact.ComponentChildren }) {
  return (
    <div class="field">
      <label>{label}{required ? <span class="req"> *</span> : null}</label>
      {children}
      {hint ? <span class="hint">{hint}</span> : null}
    </div>
  );
}

export function FieldInput({ field, value, onChange }: { field: RField; value: unknown; onChange: (value: unknown) => void }) {
  const shown = value === undefined ? '' : String(value);
  const placeholder = field.default === null || field.default === undefined || typeof field.default === 'object' ? '' : String(field.default);

  switch (field.kind) {
    case 'boolean':
      return <label class="check-inline"><input type="checkbox" checked={(value ?? field.default) === true} onChange={event => onChange((event.target as HTMLInputElement).checked)} /> {(value ?? field.default) === true ? 'yes' : 'no'}</label>;

    case 'choice':
    case 'movement':
      return (
        <select class="input" value={shown} onChange={event => { const v = (event.target as HTMLSelectElement).value; onChange(v === '' ? undefined : v); }}>
          <option value="">{field.required ? 'Choose…' : placeholder ? `(default: ${placeholder})` : '(default)'}</option>
          {field.options.map(option => <option key={option} value={option}>{option}</option>)}
        </select>
      );

    case 'integer':
    case 'number':
      return (
        <input
          class="input"
          inputMode="decimal"
          placeholder={placeholder}
          title={field.min !== null ? `${field.min} to ${field.max}` : undefined}
          value={shown}
          onInput={event => {
            const raw = (event.target as HTMLInputElement).value.trim();
            const number = Number(raw);

            if (raw === '') onChange(undefined);
            else if (Number.isFinite(number)) onChange(field.kind === 'integer' ? Math.round(number) : number);
          }}
        />
      );

    case 'colour':
      return (
        <div class="row">
          <input type="color" class="swatch" value={typeof value === 'string' ? value : typeof field.default === 'string' ? field.default : '#ffffff'} onInput={event => onChange((event.target as HTMLInputElement).value.toUpperCase())} />
          <span class="muted">{typeof value === 'string' ? value : '(default)'}</span>
          {value !== undefined && <button class="btn ghost small" onClick={() => onChange(undefined)}>Reset</button>}
        </div>
      );

    case 'vec3': {
      const list = Array.isArray(value) ? value.map(Number) : [0, 0, 0];

      return (
        <div class="row">
          {['x', 'y', 'z'].map((axis, index) => (
            <input
              key={axis}
              class="input"
              style="width:80px"
              placeholder={axis}
              inputMode="decimal"
              value={String(list[index] ?? 0)}
              onInput={event => {
                const number = Number((event.target as HTMLInputElement).value);

                if (!Number.isFinite(number)) return;

                const next = [...list];

                next[index] = number;
                onChange(next.every(n => n === 0) ? undefined : next);
              }}
            />
          ))}
        </div>
      );
    }

    case 'object':
    case 'list':
      if (field.kind === 'list' && field.options.length > 0) {
        const chosen = Array.isArray(value) ? value.map(String) : [];

        return (
          <div class="check-list">
            {field.options.map(option => (
              <label key={option} class="check-inline">
                <input
                  type="checkbox"
                  checked={chosen.includes(option)}
                  onChange={event => {
                    const next = (event.target as HTMLInputElement).checked ? [...chosen, option] : chosen.filter(item => item !== option);

                    onChange(next.length === 0 ? undefined : next);
                  }}
                /> {option}
              </label>
            ))}
          </div>
        );
      }

      if (field.kind === 'list' || Array.isArray(field.default)) {
        return <ListInput field={field} value={value} onChange={onChange} />;
      }

      return <span class="muted">Edit this one in the JSON view.</span>;

    default:
      return <input class="input" placeholder={placeholder} value={shown} onInput={event => { const v = (event.target as HTMLInputElement).value; onChange(v === '' ? undefined : v); }} />;
  }
}

/** A list of names, written with commas between. */
function ListInput({ field, value, onChange }: { field: RField; value: unknown; onChange: (value: unknown) => void }) {
  const [draft, setDraft] = useState<string | null>(null);
  const list = Array.isArray(value) ? value.map(String) : [];

  return (
    <input
      class="input"
      placeholder="names, separated by commas"
      title={field.options.length ? field.options.join(', ') : undefined}
      value={draft ?? list.join(', ')}
      onInput={event => {
        const raw = (event.target as HTMLInputElement).value;

        setDraft(raw);

        const items = raw.split(',').map(item => item.trim()).filter(Boolean);

        onChange(items.length === 0 ? undefined : items);
      }}
      onBlur={() => setDraft(null)}
    />
  );
}

/** Her traits: one choice for each category, with the settings of the one chosen. */
function Traits({ body, reference, packTraits, archetype, onChange }: { body: Json; reference: Reference; packTraits: string[]; archetype: string; onChange: (value: Json) => void }) {
  const traits: Json = isObject(body.traits) ? body.traits : {};
  const stack = archetype === 'settler' ? 'brain' : 'goals';

  const setTraits = (next: Json) => {
    const clean: Json = {};

    for (const [key, value] of Object.entries(next)) if (value !== undefined) clean[key] = value;

    const result: Json = { ...body };

    if (Object.keys(clean).length === 0) delete result.traits;
    else result.traits = clean;

    onChange(result);
  };

  return (
    <div class="form-grid">
      {Object.values(reference.categories).map(category => {
        const choices = Object.values(reference.traits).filter(trait => trait.category === category.name && (trait.stacks.length === 0 || trait.stacks.includes(stack)));

        if (choices.length === 0) return null;

        const current = traits[category.name];

        if (category.several) {
          const chosen = (Array.isArray(current) ? current : current === undefined ? [] : [current]).map(item => short(typeof item === 'string' ? item : String((item as Json).type)));

          return (
            <Labeled key={category.name} label={category.name} hint={category.description}>
              <div class="check-list">
                {choices.map(trait => (
                  <label key={trait.id} class="check-inline" title={trait.description}>
                    <input
                      type="checkbox"
                      checked={chosen.includes(short(trait.id))}
                      onChange={event => {
                        const on = (event.target as HTMLInputElement).checked;
                        const names = on ? [...chosen, short(trait.id)] : chosen.filter(name => name !== short(trait.id));

                        setTraits({ ...traits, [category.name]: names });
                      }}
                    /> {short(trait.id)}
                  </label>
                ))}
              </div>
              <span class="hint">Leave all unchecked for the standard ones; check none on purpose with an empty list in the JSON view.</span>
            </Labeled>
          );
        }

        const name = current === undefined ? '' : short(typeof current === 'string' ? current : String((current as Json).type));
        const trait = choices.find(t => short(t.id) === name);
        const settings: Json = isObject(current) ? current : {};

        return (
          <Labeled key={category.name} label={category.name} hint={trait?.description ?? category.description}>
            <select
              class="input"
              value={name}
              onChange={event => {
                const v = (event.target as HTMLSelectElement).value;

                setTraits({ ...traits, [category.name]: v === '' ? undefined : v });
              }}
            >
              <option value="">(standard)</option>
              {choices.map(option => <option key={option.id} value={short(option.id)}>{short(option.id)}</option>)}
              {packTraits.filter(own => !choices.some(c => short(c.id) === own)).length > 0 && <option disabled>— from this pack —</option>}
              {packTraits.filter(own => !choices.some(c => short(c.id) === own)).map(own => <option key={own} value={own}>{own}</option>)}
            </select>
            {trait && trait.params.length > 0 && (
              <div class="form-grid nested">
                {trait.params.map(param => (
                  <Labeled key={param.name} label={param.name.replace(/_/g, ' ')} hint={param.description}>
                    <FieldInput
                      field={param}
                      value={settings[param.name]}
                      onChange={value => {
                        const next: Json = { ...settings, type: name };

                        if (value === undefined) delete next[param.name];
                        else next[param.name] = value;

                        setTraits({ ...traits, [category.name]: Object.keys(next).length === 1 ? name : next });
                      }}
                    />
                  </Labeled>
                ))}
              </div>
            )}
          </Labeled>
        );
      })}
    </div>
  );
}
