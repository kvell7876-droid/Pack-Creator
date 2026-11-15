import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from './api';
import { Markdown } from './markdown';
import { messageOf, notify } from './notify';
import type { RField, Reference } from './types';

/* The guides are the markdown files in tools/pack_creator/docs, put into the program when it is built. The first digits of a file name order them and are not part of its name. */
const files = import.meta.glob('../../docs/*.md', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

interface Guide {
  slug: string;
  title: string;
  text: string;
}

const guides: Guide[] = Object.entries(files)
  .sort(([a], [b]) => a.localeCompare(b))
  .map(([path, text]) => ({
    slug: /\/\d*-?([^/]+)\.md$/.exec(path)![1],
    title: /^# (.+)$/m.exec(text)?.[1] ?? path,
    text
  }));

/* The pages made from the game's own export, so they cannot drift from the game. */
const references = [
  { slug: 'ref-components', title: 'Girl components' },
  { slug: 'ref-traits', title: 'Traits' },
  { slug: 'ref-goals', title: 'Goals and capabilities' },
  { slug: 'ref-scene', title: 'Scene settings' },
  { slug: 'ref-cues', title: 'Cues and sounds' },
  { slug: 'ref-dialogue', title: 'Dialogue' }
];

function rangeOf(field: RField): string {
  if (field.options.length > 0) return field.options.join(', ');
  if (field.min !== null && field.max !== null) return `${round(field.min)} to ${round(field.max)}`;

  return field.of ? `list of ${field.of}` : '';
}

function round(value: number): string {
  return String(Math.round(value * 1000) / 1000);
}

function FieldTable({ fields }: { fields: RField[] }) {
  if (fields.length === 0) return <p class="muted">No settings.</p>;

  return (
    <table>
      <thead><tr><th>Setting</th><th>Kind</th><th>Values</th><th>Default</th><th>What it does</th></tr></thead>
      <tbody>
        {fields.map(field => (
          <tr key={field.name}>
            <td><code>{field.name}</code>{field.required && <span class="req" title="Required"> *</span>}</td>
            <td>{field.kind}</td>
            <td>{rangeOf(field)}</td>
            <td>{field.default === null || field.default === undefined ? '' : <code>{JSON.stringify(field.default)}</code>}</td>
            <td>{field.description}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function Reference({ slug, data }: { slug: string; data: Reference }) {
  switch (slug.replace(/^ref-/, '')) {
    case 'components':
      return (
        <>
          <h1>Girl components</h1>
          <p>A girl file is a set of components. The types below each need certain components; a <span class="req">*</span> marks a required setting.</p>
          <h2>Girl types</h2>
          <ul>{Object.entries(data.archetypes).map(([id, needs]) => <li key={id}><code>{id}</code> needs {needs.map((n, i) => <span key={n}>{i > 0 && ', '}<code>{n}</code></span>)}</li>)}</ul>
          {Object.entries(data.components).map(([id, component]) => (
            <section key={id}>
              <h2><code>{id}</code></h2>
              <p>{component.description}{component.required ? ' Every girl has it.' : ''}</p>
              <FieldTable fields={component.fields} />
            </section>
          ))}
        </>
      );

    case 'traits':
      return (
        <>
          <h1>Traits</h1>
          <p>Traits are grouped by category. A category that does not take several holds one trait per girl.</p>
          {Object.values(data.categories).map(category => (
            <section key={category.name}>
              <h2>{category.name}</h2>
              <p>{category.description} {category.several ? 'A girl can have several.' : 'A girl has one.'}</p>
              {Object.values(data.traits).filter(trait => trait.category === category.name).map(trait => (
                <div key={trait.id} class="doc-entry">
                  <h3><code>{trait.id}</code></h3>
                  <p>{trait.description}</p>
                  {trait.params.length > 0 && <FieldTable fields={trait.params} />}
                </div>
              ))}
            </section>
          ))}
        </>
      );

    case 'goals':
      return (
        <>
          <h1>Goals and capabilities</h1>
          <p>What a trait file's <code>goals</code> and <code>capabilities</code> may use.</p>
          <h2>Capabilities</h2>
          <FieldTable fields={Object.values(data.capabilities)} />
          <h2>Conditions a goal may carry</h2>
          <ul>{data.goal_conditions.map(condition => <li key={condition}><code>{condition}</code></li>)}</ul>
          <h2>Goals</h2>
          {Object.entries(data.goals).map(([id, fields]) => (
            <div key={id} class="doc-entry">
              <h3><code>{id}</code></h3>
              <FieldTable fields={fields} />
            </div>
          ))}
        </>
      );

    case 'scene':
      return (
        <>
          <h1>Scene settings</h1>
          <FieldTable fields={data.scene_fields} />
          <h2>Tags</h2>
          <p>{data.taste_tags.map((tag, i) => <span key={tag}>{i > 0 && ', '}<code>{tag}</code></span>)}</p>
        </>
      );

    case 'cues':
      return (
        <>
          <h1>Cues and sounds</h1>
          <h2>Cues the game's animations fire</h2>
          <p>{(data.known_cues ?? []).map((cue, i) => <span key={cue}>{i > 0 && ', '}<code>{cue}</code></span>)}</p>
          <h2>Sound events</h2>
          <p>{(data.sound_events ?? []).map((sound, i) => <span key={sound}>{i > 0 && ', '}<code>{sound}</code></span>)}</p>
        </>
      );

    default:
      return (
        <>
          <h1>Dialogue</h1>
          <h2>Topics</h2>
          <p>{data.dialogue_topics.map((topic, i) => <span key={topic}>{i > 0 && ', '}<code>{topic}</code></span>)}</p>
          <h2>Ids of the game's own lines</h2>
          <p>{data.dialogue_ids.map((id, i) => <span key={id}>{i > 0 && ', '}<code>{id}</code></span>)}</p>
        </>
      );
  }
}

/** The text a search looks in, for a page. */
function searchable(slug: string, data: Reference | null): string {
  const guide = guides.find(g => g.slug === slug);

  if (guide) return guide.text.toLowerCase();
  if (!data) return '';

  switch (slug.replace(/^ref-/, '')) {
    case 'components': return JSON.stringify([data.components, data.archetypes]).toLowerCase();
    case 'traits': return JSON.stringify([data.traits, data.categories]).toLowerCase();
    case 'goals': return JSON.stringify([data.goals, data.capabilities, data.goal_conditions]).toLowerCase();
    case 'scene': return JSON.stringify([data.scene_fields, data.taste_tags]).toLowerCase();
    case 'cues': return JSON.stringify([data.known_cues, data.sound_events]).toLowerCase();
    default: return JSON.stringify([data.dialogue_topics, data.dialogue_ids]).toLowerCase();
  }
}

/**
 * The docs: guides written for pack makers, and reference pages made from the game's own export (so the lists of components, traits, goals, scene settings, cues and sounds are always
 * what this version of the game has). A link written `doc:name` or `ref:name` in a guide goes to another page.
 */
export function DocsDialog({ onClose, start, onTour }: { onClose: () => void; start?: string; onTour?: () => void }) {
  const [page, setPage] = useState(start ?? guides[0].slug);
  const [query, setQuery] = useState('');
  const [data, setData] = useState<Reference | null>(null);

  useEffect(() => { api.reference().then(setData).catch(error => notify(messageOf(error))); }, []);

  const go = (target: string) => setPage(target.startsWith('ref:') ? 'ref-' + target.slice(4) : target.slice(4));

  const needle = query.trim().toLowerCase();
  const matches = (slug: string) => !needle || searchable(slug, data).includes(needle);

  const body = useMemo(() => {
    const guide = guides.find(g => g.slug === page);

    if (guide) return <Markdown text={guide.text} go={go} />;

    return data ? <Reference slug={page} data={data} /> : <p class="muted">Loading…</p>;
  }, [page, data]);

  return (
    <div class="scrim docs-scrim" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }} onKeyDown={event => { if (event.key === 'Escape') onClose(); }}>
      <div class="dialog docs" role="dialog" aria-label="Documentation">
        <div class="docs-nav">
          <input class="input" autoFocus placeholder="Search the docs" value={query} onInput={e => setQuery((e.target as HTMLInputElement).value)} />
          <div class="docs-group">Guides</div>
          {guides.filter(g => matches(g.slug)).map(g => <button key={g.slug} class={'docs-item' + (g.slug === page ? ' on' : '')} onClick={() => setPage(g.slug)}>{g.title}</button>)}
          <div class="docs-group">Reference (from the game)</div>
          {references.filter(r => matches(r.slug)).map(r => <button key={r.slug} class={'docs-item' + (r.slug === page ? ' on' : '')} onClick={() => setPage(r.slug)}>{r.title}</button>)}
          <div class="foot">{onTour && <button class="btn ghost" onClick={onTour}>Take the tour</button>}<button class="btn ghost" onClick={onClose}>Close</button></div>
        </div>
        <div class="docs-page">{body}</div>
      </div>
    </div>
  );
}
