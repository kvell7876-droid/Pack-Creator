import { getSettings, onSettings, setSettings } from './settings';

/** Browser-style interface zoom (Ctrl + / Ctrl - / Ctrl 0 and Ctrl + wheel), kept in the settings. It scales the whole window, so a large or high-density screen can have bigger text and controls. */

const STEP = 0.1;
const MIN = 0.7;
const MAX = 2;

async function apply(factor: number): Promise<void> {
  try {
    const { getCurrentWebview } = await import('@tauri-apps/api/webview');

    await getCurrentWebview().setZoom(factor);
  } catch {
    /* Outside the app (the page opened in a browser) there is nothing to zoom; the browser's own zoom does it. */
  }
}

const round = (value: number) => Math.round(Math.max(MIN, Math.min(MAX, value)) * 100) / 100;

export function initZoom(): void {
  void apply(getSettings().uiZoom);
  onSettings(settings => void apply(settings.uiZoom));

  window.addEventListener('keydown', event => {
    if (!(event.ctrlKey || event.metaKey) || event.altKey) return;

    const current = getSettings().uiZoom;

    if (event.key === '=' || event.key === '+') setSettings({ uiZoom: round(current + STEP) });
    else if (event.key === '-' || event.key === '_') setSettings({ uiZoom: round(current - STEP) });
    else if (event.key === '0') setSettings({ uiZoom: 1 });
    else return;

    event.preventDefault();
  });

  window.addEventListener('wheel', event => {
    if (!event.ctrlKey) return;

    event.preventDefault();
    setSettings({ uiZoom: round(getSettings().uiZoom + (event.deltaY < 0 ? STEP : -STEP)) });
  }, { passive: false });
}
