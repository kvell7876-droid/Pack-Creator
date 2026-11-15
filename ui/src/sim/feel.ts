import type { ClipData } from '../types';
import { DEFAULT_PACE, type PaceSettings } from './pace';
import type { SceneCatalogue } from './roles';

/*
 * The "feel" of a scene, read from the scene itself, so the pace game does not ask the same of every scene. A scene whose clips are long and slow gets longer stretches and
 * clips that are held longer before they change; a scene with a quick beat fills in the time its own thrust cues would have filled the bar in the classic game; a scene with
 * no fast clip has no fast stretch at all. Anything the scene's author sets in its `minigame` block still wins over what is read here.
 */

const PROGRESS_PER_CUE = 0.1;

export interface Feel {
  settings: PaceSettings;
  /** What was read, in a sentence, for the person to see. */
  summary: string;
}

const clamp = (value: number, low: number, high: number): number => Math.min(high, Math.max(low, value));

/** The seconds from one thrust cue to the next in a clip that repeats, or null when it has none. */
function beatOf(clip: ClipData | null): number | null {
  if (!clip || clip.duration <= 0) return null;

  const beats = clip.events.filter(e => e.kind === 'instruction' && e.payload.toLowerCase().includes('thrust')).length;

  return beats > 0 ? clip.duration / beats : null;
}

export function deriveFeel(catalogue: SceneCatalogue, clipNamed: (name: string) => ClipData | null, cumThreshold: number, usesKeyframeEvents: boolean): Feel {
  const slow = clipNamed(catalogue.of('slow')[0] ?? catalogue.of('slow_L')[0] ?? '');
  const fast = clipNamed(catalogue.of('fast')[0] ?? '');
  const slowBeat = beatOf(slow);
  const fastBeat = beatOf(fast);
  const hasFast = fast !== null;
  const notes: string[] = [];

  /* The stretches are whole loops of the clip she is playing in them. */
  const slowSeconds = slow ? clamp(Math.round(slow.duration * 3), 10, 30) : DEFAULT_PACE.slowSeconds;
  const fastSeconds = hasFast ? clamp(Math.round(fast.duration * 4), 6, 16) : 0;

  /* The classic game fills the bar with a tenth for each thrust cue, so a scene's own beat says how long it is meant to take; the pace game also spends time slow. */
  const beat = fastBeat ?? slowBeat ?? 1;
  const fillSeconds = clamp(Math.round((cumThreshold / PROGRESS_PER_CUE) * beat * 1.25), 40, 150);

  /* A clip is never changed mid-loop if it can be helped: it is kept for about half a loop. */
  const longest = Math.max(slow?.duration ?? 0, fast?.duration ?? 0);
  const minClipSeconds = clamp(longest * 0.6, 1.2, 3.2);

  if (!hasFast) notes.push('no fast clip, so only the slow pace');
  if (usesKeyframeEvents) notes.push('the clips change on the scene\'s own cues');

  const summary = `${slowBeat ? `Slow beat ${slowBeat.toFixed(1)} s` : 'Slow clip has no thrust cue'}${hasFast ? (fastBeat ? `, fast ${fastBeat.toFixed(1)} s` : ', fast has no thrust cue') : ''}. Fills in about ${fillSeconds} s${hasFast ? `, slow stretches of ${slowSeconds} s and fast of ${fastSeconds} s` : ''}${notes.length ? `; ${notes.join('; ')}` : ''}.`;

  return {
    settings: {
      ...DEFAULT_PACE,
      slowSeconds,
      fastSeconds,
      fillSeconds,
      minClipSeconds,
      hasFast,
      /* Without a fast clip she wants one steady pace, and the band is wide. */
      slowBand: hasFast ? DEFAULT_PACE.slowBand : [0.2, 0.7]
    },
    summary
  };
}
