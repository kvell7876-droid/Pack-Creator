import { useState } from 'preact/hooks';

type Zoom = 'fit' | number;
type Background = 'checker' | 'dark' | 'light';

const ZOOMS: Zoom[] = ['fit', 1, 2, 4, 8];

function bytes(size: number): string {
  return size >= 1048576 ? `${(size / 1048576).toFixed(1)} MiB` : `${Math.max(1, Math.round(size / 1024))} KiB`;
}

/**
 * A picture of the pack (a texture): fitted to the panel by default, or at a zoom you choose, with pixels kept sharp (a texture is made of them) and a background that
 * shows what is transparent. It is drawn inside its own scrolling area, so a large texture never spills over the panels around it.
 */
export function ImageView({ src, rel, size }: { src: string; rel: string; size?: number }) {
  const [zoom, setZoom] = useState<Zoom>('fit');
  const [background, setBackground] = useState<Background>('checker');
  const [natural, setNatural] = useState<[number, number] | null>(null);

  const style = zoom === 'fit' || !natural ? undefined : { width: natural[0] * zoom, height: natural[1] * zoom };

  return (
    <div class="imageview">
      <div class="imagebar">
        <span class="muted">{rel.slice(rel.lastIndexOf('/') + 1)}</span>
        {natural && <span class="muted">{natural[0]} x {natural[1]}{size ? ` · ${bytes(size)}` : ''}</span>}
        <span class="grow" />
        <span class="seg-group">
          {ZOOMS.map(z => <button key={String(z)} class={'seg' + (zoom === z ? ' on' : '')} onClick={() => setZoom(z)}>{z === 'fit' ? 'Fit' : `${z * 100}%`}</button>)}
        </span>
        <select class="input small-select" value={background} title="What is behind the picture" onChange={e => setBackground((e.target as HTMLSelectElement).value as Background)}>
          <option value="checker">Checkerboard</option>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
      </div>
      <div class={'imagearea bg-' + background + (zoom === 'fit' ? ' fit' : '')}>
        <img src={src} alt={rel} style={style} onLoad={e => setNatural([(e.target as HTMLImageElement).naturalWidth, (e.target as HTMLImageElement).naturalHeight])} />
      </div>
    </div>
  );
}
