import { useEffect, useState } from 'preact/hooks';
import { api, pickFolder } from './api';
import { messageOf, notify } from './notify';
import type { Opened } from './types';

/** The folder name a pack of this name gets (Windows does not allow some characters in a folder name). Mirrors the core. */
function folderName(name: string): string {
  return name.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '').trim().replace(/\.+$/, '');
}

export function NewProject({ onCancel, onCreated }: { onCancel: () => void; onCreated: (opened: Opened) => void }) {
  const [step, setStep] = useState(0);
  const [name, setName] = useState('');
  const [parent, setParent] = useState('');
  const [namespace, setNamespace] = useState('pleasurehorizons');
  const [author, setAuthor] = useState('');
  const [description, setDescription] = useState('');
  const [contents, setContents] = useState<string[]>(['girls']);
  const [options, setOptions] = useState<{ id: string; label: string }[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void api.defaultProjectsDir().then(setParent);
    void api.contentsOptions().then(setOptions);
  }, []);

  useEffect(() => {
    let current = true;

    void api.namespaceProblem(namespace).then(result => { if (current) setProblem(result); });
    return () => { current = false; };
  }, [namespace]);

  const folder = folderName(name);
  const detailsOk = folder.length > 0 && parent.length > 0 && problem === null;

  const browse = async () => {
    const chosen = await pickFolder('Where should the pack be made?', parent);

    if (chosen) setParent(chosen);
  };

  const create = async () => {
    setBusy(true);

    try {
      onCreated(await api.createProject({ name, parent, namespace, author, description, contents }));
    } catch (error) {
      notify(messageOf(error));
      setBusy(false);
    }
  };

  const toggle = (id: string) => setContents(list => (list.includes(id) ? list.filter(c => c !== id) : [...list, id]));

  return (
    <div class="wizard">
      <div class="wizard-card">
        <h2>New project</h2>
        <div class="muted">{['Name it', 'What it holds', 'Review'][step]}</div>
        <div class="steps">{[0, 1, 2].map(i => <span key={i} class={i <= step ? 'on' : ''} />)}</div>

        {step === 0 && (
          <>
            <div class="field">
              <label for="pname">Name</label>
              <input id="pname" class="input" autoFocus value={name} placeholder="My Pack" onInput={e => setName((e.target as HTMLInputElement).value)} />
              <span class="hint">A folder with this name is made for the pack.</span>
            </div>
            <div class="field">
              <label for="ploc">Where</label>
              <div class="row">
                <input id="ploc" class="input grow" value={parent} onInput={e => setParent((e.target as HTMLInputElement).value)} />
                <button class="btn" onClick={browse}>Browse…</button>
              </div>
            </div>
            <div class="field">
              <label for="pns">Namespace</label>
              <input id="pns" class={'input' + (problem ? ' bad' : '')} value={namespace} onInput={e => setNamespace((e.target as HTMLInputElement).value)} />
              {problem ? <span class="problem">{problem}</span> : <span class="hint">Where the pack's data lives (data/{namespace || '…'}/). Keep pleasurehorizons for a girl who uses the game's own paths.</span>}
            </div>
            <div class="row">
              <div class="field grow">
                <label for="pauthor">Author</label>
                <input id="pauthor" class="input" value={author} onInput={e => setAuthor((e.target as HTMLInputElement).value)} />
              </div>
              <div class="field grow">
                <label for="pdesc">Description</label>
                <input id="pdesc" class="input" value={description} onInput={e => setDescription((e.target as HTMLInputElement).value)} />
              </div>
            </div>
          </>
        )}

        {step === 1 && (
          <>
            <p class="muted" style="margin-top:0">Choose what you plan to put in the pack. A pack can hold any of these, one of them or all, and you can add more at any time.</p>
            <div class="check-list">
              {options.map(option => (
                <label key={option.id} class="check">
                  <input type="checkbox" checked={contents.includes(option.id)} onChange={() => toggle(option.id)} />
                  <span>{option.label}</span>
                </label>
              ))}
            </div>
          </>
        )}

        {step === 2 && (
          <dl class="summary">
            <dt>Name</dt><dd>{name.trim()}</dd>
            <dt>Folder</dt><dd>{parent.replace(/[\\/]+$/, '')}/{folder}</dd>
            <dt>Namespace</dt><dd>{namespace}</dd>
            <dt>Author</dt><dd>{author || '—'}</dd>
            <dt>Holds</dt><dd>{contents.length ? contents.join(', ') : 'nothing yet'}</dd>
          </dl>
        )}

        <div class="wizard-foot">
          <button class="btn ghost" onClick={step === 0 ? onCancel : () => setStep(step - 1)}>{step === 0 ? 'Cancel' : 'Back'}</button>
          {step < 2
            ? <button class="btn primary" disabled={step === 0 && !detailsOk} onClick={() => setStep(step + 1)}>Next</button>
            : <button class="btn primary" disabled={busy} onClick={create}>Create project</button>}
        </div>
      </div>
    </div>
  );
}
