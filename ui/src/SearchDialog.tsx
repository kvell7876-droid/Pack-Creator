import { useEffect, useRef, useState } from 'preact/hooks';
import { api } from './api';
import { messageOf, notify } from './notify';
import type { SearchHit } from './types';

/**
 * Find (and replace) text in every file of the pack. Results are grouped by file; a click opens the file at the match. Replacing needs the replacement typed and is confirmed with
 * the number of places it will change; changed files that are open reload by themselves.
 */
export function SearchDialog({ onOpen, onClose, onReplaced }: { onOpen: (rel: string, at: { line: number; column: number }) => void; onClose: () => void; onReplaced: () => void }) {
  const [query, setQuery] = useState('');
  const [replacement, setReplacement] = useState('');
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [searched, setSearched] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  /* Searches a moment after typing stops. */
  useEffect(() => {
    window.clearTimeout(timer.current);
    setConfirm(false);

    if (!query) { setHits([]); setSearched(false); return; }

    timer.current = window.setTimeout(() => {
      void api.searchText(query, caseSensitive).then(found => { setHits(found); setSearched(true); }).catch(error => notify(messageOf(error)));
    }, 250);

    return () => window.clearTimeout(timer.current);
  }, [query, caseSensitive]);

  const files = [...new Set(hits.map(h => h.rel))];

  const replaceAll = async () => {
    try {
      const count = await api.replaceInFiles(query, replacement, caseSensitive, files);

      notify(`Replaced ${count} place${count === 1 ? '' : 's'} in ${files.length} file${files.length === 1 ? '' : 's'}. Each file's earlier version is kept in its History.`, true);
      onReplaced();
      onClose();
    } catch (error) {
      notify(messageOf(error));
    }
  };

  return (
    <div class="scrim" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }} onKeyDown={event => { if (event.key === 'Escape') onClose(); }}>
      <div class="dialog wide" role="dialog" aria-label="Find in files">
        <h3>Find in files</h3>
        <div class="row">
          <input class="input grow" autoFocus placeholder="Find" value={query} onInput={e => setQuery((e.target as HTMLInputElement).value)} />
          <label class="check-inline" title="Match capital letters exactly"><input type="checkbox" checked={caseSensitive} onChange={e => setCaseSensitive((e.target as HTMLInputElement).checked)} /> Aa</label>
        </div>
        <div class="row" style="margin-top:6px">
          <input class="input grow" placeholder="Replace with" value={replacement} onInput={e => setReplacement((e.target as HTMLInputElement).value)} />
          {!confirm && <button class="btn" disabled={hits.length === 0} onClick={() => setConfirm(true)}>Replace all…</button>}
          {confirm && <button class="btn primary" onClick={() => void replaceAll()}>Replace {hits.length}{hits.length >= 500 ? '+' : ''} in {files.length} file{files.length === 1 ? '' : 's'}</button>}
        </div>

        <div class="search-results">
          {searched && hits.length === 0 && <div class="muted">Nothing found.</div>}
          {files.map(file => (
            <div key={file}>
              <div class="search-file">{file} <span class="muted">{hits.filter(h => h.rel === file).length}</span></div>
              {hits.filter(h => h.rel === file).map((hit, i) => (
                <div key={i} class="search-hit" onClick={() => { onClose(); onOpen(hit.rel, { line: hit.line, column: hit.column }); }}>
                  <span class="muted">{hit.line}</span>
                  <span class="search-text">{hit.preview.slice(0, hit.start)}<mark>{hit.preview.slice(hit.start, hit.start + hit.length)}</mark>{hit.preview.slice(hit.start + hit.length)}</span>
                </div>
              ))}
            </div>
          ))}
          {hits.length >= 500 && <div class="muted">The first 500 matches are shown.</div>}
        </div>

        <div class="foot"><button class="btn ghost" onClick={onClose}>Close</button></div>
      </div>
    </div>
  );
}
