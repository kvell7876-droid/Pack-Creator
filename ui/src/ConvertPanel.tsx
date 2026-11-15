import { useState } from 'preact/hooks';
import { api, pickFile } from './api';
import { messageOf, notify } from './notify';
import type { ImportResult } from './types';

type Role = 'model_nude' | 'model_dressed' | 'animation_base' | 'animation_scene' | 'animation_keep';

const MODEL_ROLES: { id: Role; label: string }[] = [
  { id: 'model_nude', label: 'Her nude model' },
  { id: 'model_dressed', label: 'Her dressed model' }
];

const ANIMATION_ROLES: { id: Role; label: string; hint: string }[] = [
  { id: 'animation_base', label: 'Her everyday animations', hint: 'Clips are renamed animation.base.<name>: idle, walk, run and the rest she plays outside a scene.' },
  { id: 'animation_scene', label: 'The animations of one scene', hint: 'Clips are renamed animation.<scene>.<name>. Rename each to the part it plays (slow, fast, cum...) in Blockbench afterwards.' },
  { id: 'animation_keep', label: 'Keep the clip names as they are', hint: 'The file is named for its source and nothing inside is renamed.' }
];

const kindOf = (path: string) => (path.toLowerCase().endsWith('.geo.json') ? 'model' : path.toLowerCase().endsWith('.animation.json') ? 'animation' : null);
const baseName = (path: string) => path.slice(Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\')) + 1);

/**
 * Converts one plain model (`.geo.json`) or animation (`.animation.json`) file into the pack as a `.strutmodel` or `.strutanim`, with no old pack around it. The file is
 * chosen, then what it is for and which girl it belongs to, and it is written where the game reads it.
 */
export function ConvertPanel({ onDone }: { onDone: (result: ImportResult) => void }) {
  const [source, setSource] = useState('');
  const [role, setRole] = useState<Role | ''>('');
  const [girl, setGirl] = useState('');
  const [scene, setScene] = useState('');
  const [stamp, setStamp] = useState('');
  const [busy, setBusy] = useState(false);

  const kind = kindOf(source.trim());

  /* A chosen file suggests what it is for and whom. */
  const choose = (path: string) => {
    setSource(path);

    const found = kindOf(path);

    setRole(found === 'model' ? 'model_nude' : found === 'animation' ? 'animation_base' : '');

    if (!girl) setGirl(baseName(path).split('.')[0].toLowerCase().replace(/[^a-z0-9_]/g, '_'));
  };

  const browse = async (set: (path: string) => void, title: string) => {
    const picked = await pickFile(title, ['json']);

    if (picked) set(picked);
  };

  const run = async () => {
    setBusy(true);

    try {
      onDone(await api.convertFile(source.trim(), role, girl.trim(), role === 'animation_scene' ? scene.trim() : null, stamp.trim() || null));
    } catch (error) {
      notify(messageOf(error));
      setBusy(false);
    }
  };

  const roles = kind === 'model' ? MODEL_ROLES : kind === 'animation' ? ANIMATION_ROLES : [];
  const needsGirl = role !== 'animation_keep';
  const ready = kind !== null && role !== '' && (!needsGirl || /^[a-z0-9_]+$/.test(girl.trim())) && (role !== 'animation_scene' || /^[a-z0-9_]+$/.test(scene.trim()));

  return (
    <>
      <p class="muted" style="margin-top:0">Convert one plain file: a Bedrock or Geckolib model (<code>.geo.json</code>) becomes a <code>.strutmodel</code>, an animation (<code>.animation.json</code>) becomes a <code>.strutanim</code>. It is put where the game reads it, for the girl you name. Nothing already in the pack is overwritten, and the original is not changed.</p>

      <div class="field">
        <label>The file</label>
        <div class="row">
          <input class="input grow" value={source} placeholder="C:\...\cat.geo.json or cat.animation.json" onInput={e => choose((e.target as HTMLInputElement).value)} />
          <button class="btn" onClick={() => void browse(choose, 'Choose a .geo.json model or an .animation.json animation')}>Browse…</button>
        </div>
        {source.trim() && kind === null && <span class="hint">That is not a <code>.geo.json</code> or an <code>.animation.json</code> file.</span>}
      </div>

      {kind !== null && (
        <>
          <div class="field">
            <label>What it is</label>
            <select class="input" value={role} onChange={e => setRole((e.target as HTMLSelectElement).value as Role)}>
              {roles.map(r => <option key={r.id} value={r.id}>{r.label}</option>)}
            </select>
            {kind === 'animation' && <span class="hint">{ANIMATION_ROLES.find(r => r.id === role)?.hint}</span>}
          </div>

          {needsGirl && (
            <div class="field">
              <label>Which girl (her id: lower case letters, digits and _)</label>
              <input class="input" value={girl} placeholder="amber" onInput={e => setGirl((e.target as HTMLInputElement).value)} />
            </div>
          )}

          {role === 'animation_scene' && (
            <div class="field">
              <label>Which scene (its id)</label>
              <input class="input" value={scene} placeholder="doggy_custom" onInput={e => setScene((e.target as HTMLInputElement).value)} />
            </div>
          )}

          {kind === 'animation' && (
            <div class="field">
              <label>The model she was animated on (optional)</label>
              <div class="row">
                <input class="input grow" value={stamp} placeholder="C:\...\cat.geo.json" onInput={e => setStamp((e.target as HTMLInputElement).value)} />
                <button class="btn" onClick={() => void browse(setStamp, 'Choose the .geo.json the animation was made for')}>Browse…</button>
              </div>
              <span class="hint">The animation is stamped with that model's skeleton, so the game can tell if a different one is used. Without it the stamp is empty, which the game only warns about.</span>
            </div>
          )}
        </>
      )}

      <div class="foot">
        <button class="btn primary" disabled={!ready || busy} onClick={run}>{busy ? 'Converting…' : 'Convert'}</button>
      </div>
    </>
  );
}
