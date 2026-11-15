/** Which guided tours the person has already been through or skipped, kept in the page's own storage so the tour opens by itself once and then only when asked. */

export type TourName = 'home' | 'workspace';

const STORE = 'ph.tour';

function read(): Partial<Record<TourName, boolean>> {
  try {
    return JSON.parse(localStorage.getItem(STORE) ?? '{}') as Partial<Record<TourName, boolean>>;
  } catch {
    return {};
  }
}

export function tourSeen(name: TourName): boolean {
  return read()[name] === true;
}

/** Marks tours as done. Skipping marks every tour, so nothing opens by itself afterwards; the tour is still there to replay. */
export function markTours(...names: TourName[]): void {
  try {
    const seen = read();

    for (const name of names) seen[name] = true;

    localStorage.setItem(STORE, JSON.stringify(seen));
  } catch {
    /* Only not remembered: the tour then opens again next time. */
  }
}

export const ALL_TOURS: TourName[] = ['home', 'workspace'];

export interface TourStep {
  /** A CSS selector for what the step points at. Left out (or not on the page) the step is a card in the middle of the window. */
  target?: string;
  title: string;
  text: string;
  /** An extra button on the step, such as opening the guide it talks about. */
  action?: { label: string; run: () => void };
}
