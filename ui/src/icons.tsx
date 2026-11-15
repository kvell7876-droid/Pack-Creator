import type { FileKind, Severity } from './types';

/** The colour each kind of file is shown in, in the tree and the tabs. */
const COLOURS: Partial<Record<FileKind, string>> = {
  girl: '#e07bd6', scene: '#7db8f0', trait: '#e5b34a', dialogue: '#8fd4a5', girl_ai: '#e5b34a', model: '#f09a5b', animation: '#c49af0',
  texture: '#6cc08b', sound: '#f0c36b', pack_meta: '#8b8f98', lang: '#8fd4a5', event_bindings: '#7db8f0', keyframe_events: '#7db8f0', sounds_json: '#f0c36b', physics: '#9ad0d0'
};

export function FolderIcon({ open }: { open?: boolean }) {
  return (
    <svg class="ico" viewBox="0 0 16 16" fill="#8e96a8">
      {open
        ? <path d="M1.5 3.5A1 1 0 0 1 2.5 2.5h3.2l1.3 1.5h6.5a1 1 0 0 1 1 1V6H3.4a1 1 0 0 0-.95.7L1.5 11.5v-8z M3 7h11.6a.5.5 0 0 1 .48.64l-1.3 5a1 1 0 0 1-.97.74H2.7a.5.5 0 0 1-.48-.64L3 7z" />
        : <path d="M1.5 3.5a1 1 0 0 1 1-1h3.2l1.3 1.5h6.5a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1v-8.5z" />}
    </svg>
  );
}

export function FileIcon({ kind }: { kind: FileKind }) {
  const colour = COLOURS[kind] ?? '#9aa0ab';

  switch (kind) {
    case 'texture':
      return <svg class="ico" viewBox="0 0 16 16" fill="none" stroke={colour} stroke-width="1.4"><rect x="2" y="2.5" width="12" height="11" rx="1.5" /><circle cx="6" cy="6.5" r="1.2" fill={colour} /><path d="M2.5 12l3.5-3.5 2.5 2.5 2-2 3 3" /></svg>;
    case 'model':
      return <svg class="ico" viewBox="0 0 16 16" fill="none" stroke={colour} stroke-width="1.4"><path d="M8 1.8l5.5 3.1v6.2L8 14.2 2.5 11.1V4.9L8 1.8zM2.8 5l5.2 3 5.2-3M8 8v6" /></svg>;
    case 'animation':
      return <svg class="ico" viewBox="0 0 16 16" fill={colour}><path d="M5 3l8 5-8 5V3z" /></svg>;
    case 'sound':
      return <svg class="ico" viewBox="0 0 16 16" fill="none" stroke={colour} stroke-width="1.4"><path d="M2.5 6h2.5l3-2.5v9L5 10H2.5V6zM10.5 5.5a3.5 3.5 0 0 1 0 5M12.3 3.7a6 6 0 0 1 0 8.6" /></svg>;
    case 'girl':
      return <svg class="ico" viewBox="0 0 16 16" fill={colour}><circle cx="8" cy="5" r="2.8" /><path d="M2.8 14c.4-3 2.4-4.6 5.2-4.6s4.8 1.6 5.2 4.6H2.8z" /></svg>;
    case 'scene':
      return <svg class="ico" viewBox="0 0 16 16" fill="none" stroke={colour} stroke-width="1.4"><rect x="2" y="3" width="12" height="10" rx="1.5" /><path d="M2 6.5h12M5 3l1.5 3.5M9 3l1.5 3.5" /></svg>;
    case 'trait':
      return <svg class="ico" viewBox="0 0 16 16" fill={colour}><path d="M8 1.5l1.9 4 4.4.5-3.3 3 .9 4.4L8 11.2 4.1 13.4 5 9 1.7 6l4.4-.5L8 1.5z" /></svg>;
    case 'dialogue':
      return <svg class="ico" viewBox="0 0 16 16" fill="none" stroke={colour} stroke-width="1.4"><path d="M2.5 3.5h11v7h-6l-3 2.5v-2.5h-2v-7z" /></svg>;
    case 'social':
      return <svg class="ico" viewBox="0 0 16 16" fill="none" stroke={colour} stroke-width="1.4"><path d="M1.8 3h7v5h-3L3.8 9.6V8h-2V3zM7.5 9.5h6.7v4.2h-1.6v1.3l-1.6-1.3H7.5V9.5z" /></svg>;
    case 'pack_meta':
      return <svg class="ico" viewBox="0 0 16 16" fill="none" stroke={colour} stroke-width="1.4"><circle cx="8" cy="8" r="2.2" /><path d="M8 1.8v2M8 12.2v2M1.8 8h2M12.2 8h2M3.6 3.6l1.4 1.4M11 11l1.4 1.4M12.4 3.6L11 5M5 11l-1.4 1.4" /></svg>;
    default:
      return <svg class="ico" viewBox="0 0 16 16" fill="none" stroke={colour} stroke-width="1.3"><path d="M4 1.8h5.2L12.5 5v9.2H4V1.8zM9 1.9V5.2h3.4" /></svg>;
  }
}

export function Chevron({ open }: { open: boolean }) {
  return <svg class={'chev' + (open ? ' open' : '')} viewBox="0 0 16 16" fill="currentColor"><path d="M6 3.5l5 4.5-5 4.5V3.5z" /></svg>;
}

export function SeverityIcon({ severity }: { severity: Severity }) {
  return (
    <svg class={'sev ' + severity} viewBox="0 0 16 16" fill="currentColor">
      {severity === 'error' && <path d="M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM7.2 4.5h1.6v4.2H7.2V4.5zm0 5.4h1.6v1.6H7.2V9.9z" />}
      {severity === 'warning' && <path d="M8 1.6l6.6 11.6H1.4L8 1.6zM7.2 6h1.6v3.6H7.2V6zm0 4.5h1.6v1.6H7.2v-1.6z" />}
      {severity === 'info' && <path d="M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM7.2 7h1.6v4.5H7.2V7zm0-2.5h1.6v1.6H7.2V4.5z" />}
    </svg>
  );
}
