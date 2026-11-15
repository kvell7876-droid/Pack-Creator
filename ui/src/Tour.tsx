import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import type { TourStep } from './tourState';

const PAD = 6;
const GAP = 14;
const WIDTH = 340;

interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

function find(selector: string | undefined): Box | null {
  if (!selector) return null;

  const element = document.querySelector(selector);

  if (!element) return null;

  const rect = element.getBoundingClientRect();

  if (rect.width === 0 && rect.height === 0) return null;

  return { left: rect.left, top: rect.top, width: rect.width, height: rect.height };
}

/**
 * A guided tour: the window dims, the thing a step is about stays lit, and a card beside it says what it is. Next and Back step through, Esc or Skip leaves at any point. Whatever is
 * underneath cannot be clicked while it runs, so a stray click cannot change anything.
 *
 * `onSkip` is told when the person leaves early, `onDone` when they reach the end.
 */
export function Tour({ steps, onDone, onSkip }: { steps: TourStep[]; onDone: () => void; onSkip: () => void }) {
  const [index, setIndex] = useState(0);
  const [box, setBox] = useState<Box | null>(null);
  const [place, setPlace] = useState<{ left: number; top: number } | null>(null);
  const card = useRef<HTMLDivElement>(null);
  const [, bump] = useState(0);

  const step = steps[index];
  const last = index === steps.length - 1;

  const next = () => (last ? onDone() : setIndex(index + 1));
  const back = () => setIndex(Math.max(0, index - 1));

  /* The target is looked for again whenever the step changes or the window is resized. */
  useLayoutEffect(() => {
    const element = step.target ? document.querySelector(step.target) : null;

    element?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    setBox(find(step.target));
  }, [index, steps]);

  useEffect(() => {
    const onResize = () => { setBox(find(steps[index].target)); bump(n => n + 1); };

    window.addEventListener('resize', onResize);

    return () => window.removeEventListener('resize', onResize);
  }, [index, steps]);

  /* The card goes below the target if it fits, else above, else beside it, else in the middle. */
  useLayoutEffect(() => {
    const height = card.current?.offsetHeight ?? 180;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const clampX = (x: number) => Math.max(12, Math.min(vw - WIDTH - 12, x));

    if (!box) {
      setPlace({ left: (vw - WIDTH) / 2, top: Math.max(12, (vh - height) / 2) });
      return;
    }

    const centreX = box.left + box.width / 2 - WIDTH / 2;

    if (box.top + box.height + PAD + GAP + height < vh - 12) setPlace({ left: clampX(centreX), top: box.top + box.height + PAD + GAP });
    else if (box.top - PAD - GAP - height > 12) setPlace({ left: clampX(centreX), top: box.top - PAD - GAP - height });
    else if (box.left + box.width + PAD + GAP + WIDTH < vw - 12) setPlace({ left: box.left + box.width + PAD + GAP, top: Math.max(12, Math.min(vh - height - 12, box.top)) });
    else if (box.left - PAD - GAP - WIDTH > 12) setPlace({ left: box.left - PAD - GAP - WIDTH, top: Math.max(12, Math.min(vh - height - 12, box.top)) });
    else setPlace({ left: (vw - WIDTH) / 2, top: Math.max(12, (vh - height) / 2) });
  }, [box, index]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onSkip();
      else if (event.key === 'ArrowRight' || event.key === 'Enter') next();
      else if (event.key === 'ArrowLeft') back();
      else return;

      event.preventDefault();
      event.stopPropagation();
    };

    window.addEventListener('keydown', onKey, true);

    return () => window.removeEventListener('keydown', onKey, true);
  });

  return (
    <div class="tour" role="dialog" aria-label="Guided tour">
      {box ? (
        <div class="tour-spot" style={{ left: box.left - PAD, top: box.top - PAD, width: box.width + PAD * 2, height: box.height + PAD * 2 }} />
      ) : (
        <div class="tour-dim" />
      )}
      <div ref={card} class="tour-card" style={{ width: WIDTH, left: place?.left ?? -9999, top: place?.top ?? -9999 }}>
        <div class="tour-count">Step {index + 1} of {steps.length}</div>
        <h3>{step.title}</h3>
        <p>{step.text}</p>
        {step.action && <button class="btn small" onClick={step.action.run}>{step.action.label}</button>}
        <div class="tour-dots">{steps.map((_, n) => <span key={n} class={n === index ? 'on' : n < index ? 'past' : ''} />)}</div>
        <div class="tour-foot">
          <button class="btn ghost small" onClick={onSkip}>Skip tour</button>
          <span class="grow" />
          {index > 0 && <button class="btn small" onClick={back}>Back</button>}
          <button class="btn primary small" autoFocus onClick={next}>{last ? 'Finish' : 'Next'}</button>
        </div>
      </div>
    </div>
  );
}
