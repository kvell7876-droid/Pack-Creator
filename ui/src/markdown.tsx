import type { ComponentChildren } from 'preact';

/**
 * A small Markdown reader for the docs: headings, paragraphs, lists, tables, fenced code, bold, inline code and links. A link that starts `doc:` or `ref:` goes to another page of
 * the docs through `go`; anything else opens as it is. No HTML is ever taken from the text.
 */
export function Markdown({ text, go }: { text: string; go: (target: string) => void }) {
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  const out: ComponentChildren[] = [];
  let i = 0;

  const inline = (source: string, key: string): ComponentChildren[] => {
    const parts: ComponentChildren[] = [];
    const pattern = /(`[^`]+`)|(\*\*[^*]+\*\*)|(\[[^\]]+\]\([^)]+\))/g;
    let last = 0;
    let match: RegExpExecArray | null;
    let n = 0;

    while ((match = pattern.exec(source))) {
      if (match.index > last) parts.push(source.slice(last, match.index));

      const token = match[0];
      const id = `${key}-${n++}`;

      if (token.startsWith('`')) {
        parts.push(<code key={id}>{token.slice(1, -1)}</code>);
      } else if (token.startsWith('**')) {
        parts.push(<b key={id}>{inline(token.slice(2, -2), id)}</b>);
      } else {
        const [, label, href] = /\[([^\]]+)\]\(([^)]+)\)/.exec(token)!;

        parts.push(href.startsWith('doc:') || href.startsWith('ref:')
          ? <a key={id} href="#" class="doc-link" onClick={event => { event.preventDefault(); go(href); }}>{label}</a>
          : <a key={id} href={href} target="_blank" rel="noreferrer">{label}</a>);
      }

      last = match.index + token.length;
    }

    if (last < source.length) parts.push(source.slice(last));

    return parts;
  };

  const cells = (row: string) => row.trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map(cell => cell.trim());

  while (i < lines.length) {
    const line = lines[i];

    if (line.startsWith('```')) {
      const code: string[] = [];

      i++;
      while (i < lines.length && !lines[i].startsWith('```')) code.push(lines[i++]);
      i++;
      out.push(<pre key={out.length}><code>{code.join('\n')}</code></pre>);
    } else if (/^#{1,3} /.test(line)) {
      const level = line.indexOf(' ');
      const Tag = (`h${level}`) as 'h1';

      out.push(<Tag key={out.length}>{inline(line.slice(level + 1), `h${out.length}`)}</Tag>);
      i++;
    } else if (line.startsWith('|') && /^\|?\s*-+/.test(lines[i + 1] ?? '')) {
      const head = cells(line);
      const rows: string[][] = [];

      i += 2;
      while (i < lines.length && lines[i].startsWith('|')) rows.push(cells(lines[i++]));

      out.push(
        <table key={out.length}>
          <thead><tr>{head.map((cell, c) => <th key={c}>{inline(cell, `th${out.length}-${c}`)}</th>)}</tr></thead>
          <tbody>{rows.map((row, r) => <tr key={r}>{row.map((cell, c) => <td key={c}>{inline(cell, `td${out.length}-${r}-${c}`)}</td>)}</tr>)}</tbody>
        </table>
      );
    } else if (/^(- |\d+\. )/.test(line)) {
      const ordered = /^\d/.test(line);
      const items: string[] = [];

      while (i < lines.length && /^(- |\d+\. )/.test(lines[i])) items.push(lines[i++].replace(/^(- |\d+\. )/, ''));

      const rendered = items.map((item, n) => <li key={n}>{inline(item, `li${out.length}-${n}`)}</li>);

      out.push(ordered ? <ol key={out.length}>{rendered}</ol> : <ul key={out.length}>{rendered}</ul>);
    } else if (line.trim() === '') {
      i++;
    } else {
      const paragraph: string[] = [];

      while (i < lines.length && lines[i].trim() !== '' && !/^(```|#{1,3} |\||- |\d+\. )/.test(lines[i])) paragraph.push(lines[i++]);

      out.push(<p key={out.length}>{inline(paragraph.join(' '), `p${out.length}`)}</p>);
    }
  }

  return <>{out}</>;
}
