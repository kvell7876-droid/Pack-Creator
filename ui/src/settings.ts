/** The Pack Creator's settings, kept in the page's own storage (they belong to the person, not to a pack) and told to whoever listens when they change. */

export interface Settings {
  /** How long after the last edit a file is saved, in milliseconds. */
  autosaveMs: number;
  /** The editor's text size. */
  fontSize: number;
  minimap: boolean;
  wordWrap: boolean;
  /** Spaces in a tab, in the editor and when a file is formatted. */
  tabSize: number;
  /** The key that thrusts in the scene simulator, as `KeyboardEvent.code`. */
  thrustKey: string;
  /** The key that finishes once the bar is full. */
  cumKey: string;
  /** Whether the simulator plays the sounds of cues, and how loud (0 to 1). */
  simSound: boolean;
  simVolume: number;
  /** The zoom of the whole window (1 is normal). */
  uiZoom: number;
}

export const DEFAULTS: Settings = {
  autosaveMs: 800,
  fontSize: 13,
  minimap: true,
  wordWrap: false,
  tabSize: 2,
  thrustKey: 'Space',
  cumKey: 'KeyC',
  simSound: true,
  simVolume: 0.8,
  uiZoom: 1
};

const STORE = 'ph.settings';
const listeners = new Set<(settings: Settings) => void>();
let current: Settings | null = null;

const clamp = (value: unknown, low: number, high: number, fallback: number): number => (typeof value === 'number' && Number.isFinite(value) ? Math.max(low, Math.min(high, value)) : fallback);

function read(): Settings {
  try {
    const stored = JSON.parse(localStorage.getItem(STORE) ?? '{}') as Partial<Settings>;

    return {
      autosaveMs: clamp(stored.autosaveMs, 200, 10000, DEFAULTS.autosaveMs),
      fontSize: clamp(stored.fontSize, 9, 28, DEFAULTS.fontSize),
      minimap: typeof stored.minimap === 'boolean' ? stored.minimap : DEFAULTS.minimap,
      wordWrap: typeof stored.wordWrap === 'boolean' ? stored.wordWrap : DEFAULTS.wordWrap,
      tabSize: clamp(stored.tabSize, 1, 8, DEFAULTS.tabSize),
      thrustKey: typeof stored.thrustKey === 'string' && stored.thrustKey ? stored.thrustKey : DEFAULTS.thrustKey,
      cumKey: typeof stored.cumKey === 'string' && stored.cumKey ? stored.cumKey : DEFAULTS.cumKey,
      simSound: typeof stored.simSound === 'boolean' ? stored.simSound : DEFAULTS.simSound,
      simVolume: clamp(stored.simVolume, 0, 1, DEFAULTS.simVolume),
      uiZoom: clamp(stored.uiZoom, 0.7, 2, DEFAULTS.uiZoom)
    };
  } catch {
    return { ...DEFAULTS };
  }
}

export function getSettings(): Settings {
  if (!current) current = read();

  return current;
}

export function setSettings(change: Partial<Settings>): void {
  current = { ...getSettings(), ...change };

  try {
    localStorage.setItem(STORE, JSON.stringify(current));
  } catch {
    /* Only not remembered. */
  }

  for (const listener of listeners) listener(current);
}

export function onSettings(listener: (settings: Settings) => void): () => void {
  listeners.add(listener);

  return () => { listeners.delete(listener); };
}

/** A key as the person knows it: `Space`, `F`, `Left Shift`. */
export function keyName(code: string): string {
  if (code.startsWith('Key')) return code.slice(3);
  if (code.startsWith('Digit')) return code.slice(5);

  return code.replace(/([a-z])([A-Z])/g, '$1 $2');
}
