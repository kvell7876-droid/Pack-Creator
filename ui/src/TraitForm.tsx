import { FieldInput, Labeled } from './GirlForm';
import type { RField, Reference } from './types';

type Json = Record<string, unknown>;

const isObject = (value: unknown): value is Json => typeof value === 'object' && value !== null && !Array.isArray(value);
const short = (id: string) => id.slice(id.indexOf(':') + 1);
const PARAM_TYPES = ['number', 'integer', 'boolean', 'identifier'];
/** Keys the form edits; anything else in the file (brain operations) is left as it is and edited in the JSON view. */
const OWN = ['category', 'description', 'params', 'goals', 'targets', 'removes', 'capabilities', 'stacks'];

interface Props {
  text: string;
  reference: Reference;
  onChange: (text: string) => void;
  onShowJson: () => void;
}

/**
 * A trait file as a form: its category, the settings a girl may tune, the goals it adds (each with its own settings, which may be one of the trait's settings written as
 * `$name`), what it changes about her body, and the kinds of girl it is for. It writes the whole file back, as the other forms do.
 */
export function TraitForm({ text, reference, onChange, onShowJson }: Props) {
  let trait: Json | null = null;

  try {
    const parsed: unknown = JSON.parse(text);

    trait = isObject(parsed) ? parsed : null;
  } catch {
    trait = null;
  }

  if (!trait) {
    return (
      <div class="form-pane">
        <div class="form-note">
          This file is not valid JSON right now, so the form cannot read it. Fix it in the JSON view (the problems panel says where), and the form comes back.
          <div style="margin-top:10px"><button class="btn" onClick={onShowJson}>Show the JSON</button></div>
        </div>
      </div>
    );
  }

  const root = trait;
  const set = (key: string, value: unknown) => {
    const next: Json = { ...root };

    if (value === undefined || (Array.isArray(value) && value.length === 0) || (isObject(value) && Object.keys(value).length === 0 && key !== 'params')) delete next[key];
    else next[key] = value;

    onChange(JSON.stringify(next, null, 2) + '\n');
  };

  const params = isObject(root.params) ? root.params : {};
  const paramNames = Object.keys(params);
  const goalNames = Object.keys(reference.goals).map(short);
  const conditions = reference.goal_conditions.map(short);
  const stacks = Array.isArray(root.stacks) ? root.stacks.map(String) : [];
  const other = Object.keys(root).filter(key => !OWN.includes(key));

  return (
    <div class="form-pane">
      <div class="form-inner">
        <div class="form-card">
          <h3>Trait</h3>
          <div class="form-grid">
            <Labeled label="Category" required hint={reference.categories[String(root.category)]?.description ?? 'The kind of choice this trait is. A girl takes traits from each category.'}>
              <select class="input" value={String(root.category ?? '')} onChange={e => set('category', (e.target as HTMLSelectElement).value || undefined)}>
                <option value="">Choose…</option>
                {Object.keys(reference.categories).map(c => <option key={c} value={c}>{c}</option>)}
              </select>
            </Labeled>
            <Labeled label="Description" hint="What it does, in a sentence. Shown by tools.">
              <input class="input" value={String(root.description ?? '')} onInput={e => set('description', (e.target as HTMLInputElement).value || undefined)} />
            </Labeled>
            <Labeled label="For" hint="Wild girls think with goals, settlers with a brain. Leave both unticked for wild girls.">
              <div class="check-list">
                {['goals', 'brain'].map(stack => (
                  <label key={stack} class="check-inline">
                    <input type="checkbox" checked={stacks.includes(stack)} onChange={e => set('stacks', (e.target as HTMLInputElement).checked ? [...stacks, stack] : stacks.filter(s => s !== stack))} /> {stack === 'goals' ? 'wild girls (goals)' : 'settlers (brain)'}
                  </label>
                ))}
              </div>
            </Labeled>
          </div>
        </div>

        <ParamsCard params={params} onChange={value => set('params', value)} />

        {(['goals', 'targets'] as const).map(key => (
          <GoalsCard
            key={key}
            title={key === 'goals' ? 'Goals it adds' : 'Target goals it adds'}
            hint={key === 'goals' ? 'What a wild girl does: the lower the priority, the sooner.' : 'Who she picks to fight.'}
            goals={Array.isArray(root[key]) ? (root[key] as unknown[]).filter(isObject) : []}
            reference={reference}
            goalNames={goalNames}
            conditions={conditions}
            paramNames={paramNames}
            onChange={value => set(key, value)}
          />
        ))}

        <div class="form-card">
          <h3>What it changes about her body</h3>
          <div class="hint">Movement, falls, and what a player can do with her. A value can be one of the trait's settings.</div>
          <div class="form-grid">
            {Object.values(reference.capabilities).map(capability => (
              <CapabilityRow
                key={capability.name}
                field={capability}
                value={isObject(root.capabilities) ? (root.capabilities as Json)[capability.name] : undefined}
                paramNames={paramNames}
                onChange={value => {
                  const next: Json = isObject(root.capabilities) ? { ...(root.capabilities as Json) } : {};

                  if (value === undefined) delete next[capability.name];
                  else next[capability.name] = value;

                  set('capabilities', next);
                }}
              />
            ))}
          </div>
        </div>

        <div class="form-card">
          <h3>Takes out</h3>
          <div class="form-grid">
            <Labeled label="Goals it removes" hint="Goals taken out of the whole plan, whichever trait added them (names, separated by commas).">
              <input class="input" value={Array.isArray(root.removes) ? root.removes.join(', ') : ''} onInput={e => set('removes', (e.target as HTMLInputElement).value.split(',').map(v => v.trim()).filter(Boolean))} />
            </Labeled>
          </div>
        </div>

        {other.length > 0 && <div class="hint" style="padding:0 4px">This trait also has {other.join(', ')} (what it changes about a settler's brain). Edit those in the JSON view; the form leaves them as they are.</div>}
      </div>
    </div>
  );
}

function ParamsCard({ params, onChange }: { params: Json; onChange: (value: Json) => void }) {
  const names = Object.keys(params);
  const update = (name: string, next: Json | null, rename?: string) => {
    const out: Json = {};

    for (const key of names) {
      if (key === name) {
        if (next !== null) out[rename ?? key] = next;
      } else out[key] = params[key];
    }

    onChange(out);
  };

  const add = () => {
    let n = 1;

    while (names.includes(`setting_${n}`)) n++;

    onChange({ ...params, [`setting_${n}`]: { type: 'number', default: 1 } });
  };

  return (
    <div class="form-card">
      <div class="form-head"><h3>Settings a girl may tune</h3><button class="btn small" style="margin-left:auto" onClick={add}>＋ Setting</button></div>
      <div class="hint">Use one in a goal as $name. A girl's file can change its value for her.</div>
      {names.map(name => {
        const body = isObject(params[name]) ? (params[name] as Json) : {};
        const set = (key: string, value: unknown) => {
          const next = { ...body };

          if (value === undefined || value === '') delete next[key];
          else next[key] = value;

          update(name, next);
        };
        const type = String(body.type ?? 'number');

        return (
          <div class="row-card" key={name}>
            <div class="form-grid">
              <Labeled label="Name"><input class="input" value={name} onChange={e => { const v = (e.target as HTMLInputElement).value.trim(); if (v && v !== name && !names.includes(v)) update(name, body, v); }} /></Labeled>
              <Labeled label="Type">
                <select class="input" value={type} onChange={e => set('type', (e.target as HTMLSelectElement).value)}>{PARAM_TYPES.map(t => <option key={t} value={t}>{t}</option>)}</select>
              </Labeled>
              <Labeled label="Default" required>
                {type === 'boolean'
                  ? <label class="check-inline"><input type="checkbox" checked={body.default === true} onChange={e => set('default', (e.target as HTMLInputElement).checked)} /> {body.default === true ? 'yes' : 'no'}</label>
                  : <input class="input" inputMode={type === 'identifier' ? undefined : 'decimal'} value={String(body.default ?? '')} onInput={e => { const raw = (e.target as HTMLInputElement).value; set('default', type === 'identifier' ? raw : raw.trim() === '' ? undefined : Number.isFinite(Number(raw)) ? Number(raw) : body.default); }} />}
              </Labeled>
              {(type === 'number' || type === 'integer') && (
                <>
                  <Labeled label="Lowest"><input class="input" inputMode="decimal" value={String(body.min ?? '')} onInput={e => { const raw = (e.target as HTMLInputElement).value; set('min', raw.trim() === '' ? undefined : Number.isFinite(Number(raw)) ? Number(raw) : body.min); }} /></Labeled>
                  <Labeled label="Highest"><input class="input" inputMode="decimal" value={String(body.max ?? '')} onInput={e => { const raw = (e.target as HTMLInputElement).value; set('max', raw.trim() === '' ? undefined : Number.isFinite(Number(raw)) ? Number(raw) : body.max); }} /></Labeled>
                </>
              )}
              <Labeled label="Description"><input class="input" value={String(body.description ?? '')} onInput={e => set('description', (e.target as HTMLInputElement).value)} /></Labeled>
            </div>
            <button class="btn ghost small" onClick={() => update(name, null)}>Remove</button>
          </div>
        );
      })}
    </div>
  );
}

interface GoalsProps {
  title: string;
  hint: string;
  goals: Json[];
  reference: Reference;
  goalNames: string[];
  conditions: string[];
  paramNames: string[];
  onChange: (value: Json[]) => void;
}

function GoalsCard({ title, hint, goals, reference, goalNames, conditions, paramNames, onChange }: GoalsProps) {
  const replace = (index: number, next: Json | null) => onChange(goals.flatMap((g, i) => (i === index ? (next === null ? [] : [next]) : [g])));
  const nextPriority = goals.reduce((n, g) => Math.max(n, Number(g.priority) || 0), 0) + 1;

  return (
    <div class="form-card">
      <div class="form-head"><h3>{title}</h3><button class="btn small" style="margin-left:auto" onClick={() => onChange([...goals, { priority: nextPriority, goal: goalNames[0] ?? '' }])}>＋ Goal</button></div>
      <div class="hint">{hint}</div>
      {goals.map((goal, index) => {
        const name = String(goal.goal ?? '');
        const fields: RField[] = reference.goals[Object.keys(reference.goals).find(id => short(id) === name) ?? ''] ?? [];
        const set = (key: string, value: unknown) => {
          const next = { ...goal };

          if (value === undefined || value === '') delete next[key];
          else next[key] = value;

          replace(index, next);
        };

        return (
          <div class="row-card" key={index}>
            <div class="form-grid">
              <Labeled label="Goal" required hint={reference.goals[Object.keys(reference.goals).find(id => short(id) === name) ?? '']?.length === 0 ? undefined : undefined}>
                <select class="input" value={name} onChange={e => replace(index, { priority: goal.priority, goal: (e.target as HTMLSelectElement).value, ...(goal.when ? { when: goal.when } : {}) })}>
                  {!goalNames.includes(name) && <option value={name}>{name || 'Choose…'}</option>}
                  {goalNames.map(g => <option key={g} value={g}>{g}</option>)}
                </select>
              </Labeled>
              <Labeled label="Priority" required hint="The lower, the sooner.">
                <input class="input" inputMode="numeric" value={String(goal.priority ?? '')} onInput={e => { const n = Number((e.target as HTMLInputElement).value); if (Number.isFinite(n)) set('priority', Math.round(n)); }} />
              </Labeled>
              <Labeled label="Only when">
                <select class="input" value={String(goal.when ?? '')} onChange={e => set('when', (e.target as HTMLSelectElement).value || undefined)}>
                  <option value="">(always)</option>
                  {conditions.map(c => <option key={c} value={c}>{c}</option>)}
                </select>
              </Labeled>
              {fields.map(field => (
                <Labeled key={field.name} label={field.name.replace(/_/g, ' ')} hint={field.description}>
                  <Linked field={field} value={goal[field.name]} paramNames={paramNames} onChange={value => set(field.name, value)} />
                </Labeled>
              ))}
            </div>
            <button class="btn ghost small" onClick={() => replace(index, null)}>Remove</button>
          </div>
        );
      })}
    </div>
  );
}

/** A setting's value, or one of the trait's own settings written `$name`. */
function Linked({ field, value, paramNames, onChange }: { field: RField; value: unknown; paramNames: string[]; onChange: (value: unknown) => void }) {
  const linked = typeof value === 'string' && value.startsWith('$');

  return (
    <div class="row">
      {linked
        ? <span class="chip" style="padding:5px 10px">{String(value)}</span>
        : <div class="grow"><FieldInput field={field} value={value} onChange={onChange} /></div>}
      {paramNames.length > 0 && (
        <select class="input small-select" title="Use one of this trait's settings" value={linked ? String(value) : ''} onChange={e => { const v = (e.target as HTMLSelectElement).value; onChange(v === '' ? undefined : v); }}>
          <option value="">{linked ? '(a value)' : '= setting…'}</option>
          {paramNames.map(p => <option key={p} value={'$' + p}>{'$' + p}</option>)}
        </select>
      )}
    </div>
  );
}

function CapabilityRow({ field, value, paramNames, onChange }: { field: RField; value: unknown; paramNames: string[]; onChange: (value: unknown) => void }) {
  const on = value !== undefined;
  const fill: RField = { ...field, kind: field.kind === 'movement' || field.options.length > 0 ? 'choice' : field.kind };

  return (
    <div class="field">
      <label class="check-inline">
        <input type="checkbox" checked={on} onChange={e => onChange((e.target as HTMLInputElement).checked ? (field.kind === 'boolean' ? true : field.options[0] ?? '') : undefined)} /> <b>{field.name.replace(/_/g, ' ')}</b>
      </label>
      <span class="hint">{field.description}</span>
      {on && <Linked field={fill} value={value} paramNames={paramNames} onChange={v => onChange(v === undefined ? (field.kind === 'boolean' ? true : '') : v)} />}
    </div>
  );
}
