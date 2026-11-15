import { useEffect, useState } from 'preact/hooks';

type Listener = (toast: { text: string; good: boolean } | null) => void;

let listener: Listener | null = null;
let timer: number | undefined;

/** Shows a message at the bottom of the window for a few seconds. */
export function notify(text: string, good = false): void {
  listener?.({ text, good });
  window.clearTimeout(timer);
  timer = window.setTimeout(() => listener?.(null), good ? 2500 : 7000);
}

export function Toaster() {
  const [toast, setToast] = useState<{ text: string; good: boolean } | null>(null);

  useEffect(() => {
    listener = setToast;
    return () => { listener = null; };
  }, []);

  return toast ? <div class={'toast' + (toast.good ? ' good' : '')} onClick={() => setToast(null)}>{toast.text}</div> : null;
}

export function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
