/*
 * A prototype of a different sex mini game: instead of holding a button until a bar is full, the player sets a pace and keeps it where she likes it, while keeping their own
 * excitement from running away. One input does it all (hold to speed up, let go to coast), so it can be played with one hand. The pace still chooses the slow or the fast clip.
 *
 * - The pace rises while the input is held. When it is let go the pace holds for a moment and then eases off slowly, so tapping keeps it steady instead of throwing it about.
 * - She wants slow for a while, then fast for a while (her band moves on a schedule the player can read). In her band her pleasure builds at full speed, in the fast band
 *   faster; outside it slowly; far above it she is overwhelmed and it falls.
 * - The player's own excitement (the edge) rises when the pace is high and cools when it is low. If it fills before she is ready, it ends early.
 * - Pressing on the beat of the clip's `thrust` cues builds a combo, which adds to her pleasure.
 *
 * Nothing here knows the scene or the clips; the engine feeds it the time and takes the numbers.
 */

export interface PaceSettings {
  /** Where she likes the pace when she wants it slow, and when she wants it fast (pace values from 0 to 1). */
  slowBand: [number, number];
  fastBand: [number, number];
  /** How long each stretch lasts, in seconds: slow, then fast, then slow again. */
  slowSeconds: number;
  fastSeconds: number;
  /** How long a change from one band to the other takes, in seconds. */
  blendSeconds: number;
  /** How long in the band it takes to fill her bar from empty, in seconds. */
  fillSeconds: number;
  /** How long a clip is kept once it has changed, in seconds. */
  minClipSeconds: number;
  /** Whether the scene has a fast clip at all (without one she only ever plays the slow). */
  hasFast: boolean;
}

export const DEFAULT_PACE: PaceSettings = { slowBand: [0.14, 0.42], fastBand: [0.6, 0.9], slowSeconds: 16, fastSeconds: 9, blendSeconds: 3, fillSeconds: 80, minClipSeconds: 1.6, hasFast: true };

/* How the pace moves. It rises while held, holds for a moment when let go, then eases off slowly (a tap or two keeps it where it is). */
const RISE = 0.7;
const GRACE = 0.4;
const FALL = 0.3;
/* The pace at which she goes to the fast clip, and back (apart, and a clip is kept at least a moment, so she never flickers between them). */
const FAST_ABOVE = 0.52;
const SLOW_BELOW = 0.46;
/* How far from a thrust cue (seconds) a press counts as on the beat. */
export const BEAT_WINDOW = 0.2;
/* The combo adds this much to her pleasure for each hit in a row, up to this many. */
const COMBO_STEP = 0.06;
const MAX_COMBO = 10;

export type Zone = 'cold' | 'slow' | 'good' | 'over';
export type Ending = 'together' | 'good' | 'early';

export class PaceGame {
  /** Where the input has taken the pace (the same as `pace`; the rules have no lag, only the drawing of the gauge trails it). */
  target = 0;
  pace = 0;
  held = false;
  fast = false;
  zone: Zone = 'cold';
  /** The player's own excitement, from 0 to 1. At 1 they cannot hold on. */
  edge = 0;
  combo = 0;
  /** How the rhythm went: the presses that were on a thrust cue, the cues there were, and the longest run of hits. */
  hits = 0;
  cues = 0;
  best = 0;
  /** An analog input (a trigger or a stick, 0 to 1), or null when there is only the key. It sets the target pace directly. */
  analog: number | null = null;
  /** Seconds since the input was let go. */
  private released = 0;
  /** Seconds since the clip last changed. */
  private clipFor = 0;

  constructor(readonly settings: PaceSettings = DEFAULT_PACE) {}

  /** Which band she wants at a time (seconds since the scene began), and how far along a change to the other one it is. */
  private phase(seconds: number): number {
    const { slowSeconds, fastSeconds, blendSeconds } = this.settings;

    const cycle = slowSeconds + fastSeconds;
    const t = ((seconds % cycle) + cycle) % cycle;
    /* How fast she wants it, from 0 (the slow band) to 1 (the fast one): a change to the fast band takes the blend before the fast stretch, and back after it. */
    if (!this.settings.hasFast || fastSeconds <= 0) return 0;
    if (t < slowSeconds - blendSeconds) return 0;
    if (t < slowSeconds) return (t - (slowSeconds - blendSeconds)) / blendSeconds;
    if (t < cycle - blendSeconds) return 1;

    return 1 - (t - (cycle - blendSeconds)) / blendSeconds;
  }

  /** Her band at a time. */
  spot(seconds: number): { lo: number; hi: number } {
    const mix = this.phase(seconds);
    const [sl, sh] = this.settings.slowBand;
    const [fl, fh] = this.settings.fastBand;

    return { lo: sl + (fl - sl) * mix, hi: sh + (fh - sh) * mix };
  }

  /** What she says to the player about the pace, or nothing when it is right. */
  hint(): string {
    return this.zone === 'slow' || this.zone === 'cold' ? 'Faster...' : this.zone === 'over' ? 'Slow down...' : '';
  }

  reset(): void {
    this.target = 0;
    this.pace = 0;
    this.held = false;
    this.fast = false;
    this.zone = 'cold';
    this.edge = 0;
    this.combo = 0;
    this.hits = 0;
    this.cues = 0;
    this.best = 0;
    this.analog = null;
    this.released = 0;
    this.clipFor = 0;
  }

  /** The input was pressed: whether it was on the beat of a cue that fired `since` seconds ago (or fires `until` seconds from now, as a negative). */
  beat(offset: number): boolean {
    const hit = Math.abs(offset) <= BEAT_WINDOW;

    this.combo = hit ? this.combo + 1 : 0;

    if (hit) {
      this.hits++;
      this.best = Math.max(this.best, this.combo);
    }

    return hit;
  }

  /** The bonus the combo gives to her pleasure, as a fraction (0.3 is 30 percent more). */
  comboBonus(): number {
    return Math.min(this.combo, MAX_COMBO) * COMBO_STEP;
  }

  /** What the scene is worth, for the rewards it gives: more for a better ending, and up to half as much again for good rhythm. */
  reward(ending: Ending): number {
    if (ending === 'early') return 0;

    const base = ending === 'together' ? 1.5 : 1;

    return Math.round((base + 0.5 * (Math.min(this.best, 12) / 12)) * 100) / 100;
  }

  /**
   * One step of `dt` seconds. `inSex` is whether the scene is at the point where the pace counts. Gives the change in her pleasure, in units of the scene's bar, for a bar
   * of `threshold`.
   */
  step(dt: number, inSex: boolean, seconds: number, threshold: number): number {
    if (this.analog !== null) {
      /* A trigger or stick says the pace outright; the target follows it quickly. */
      this.released = 0;
      this.target += (this.analog - this.target) * Math.min(1, 5 * dt);
    } else if (this.held) {
      this.released = 0;
      this.target = Math.min(1, this.target + RISE * dt);
    } else {
      this.released += dt;

      if (this.released > GRACE) this.target = Math.max(0, this.target - FALL * dt);
    }

    /* The pace is where the input has taken it: no lag in the rules, so what the player does is what the game does. The gauge trails it a little when it is drawn. */
    this.pace = this.target;
    this.clipFor += dt;

    if (this.clipFor >= this.settings.minClipSeconds && this.settings.hasFast) {
      if (!this.fast && this.pace > FAST_ABOVE) { this.fast = true; this.clipFor = 0; }
      else if (this.fast && this.pace < SLOW_BELOW) { this.fast = false; this.clipFor = 0; }
    }

    const { lo, hi } = this.spot(seconds);
    const rate = threshold / this.settings.fillSeconds;

    this.zone = this.pace < 0.08 ? 'cold' : this.pace < lo ? 'slow' : this.pace <= hi ? 'good' : 'over';

    if (!inSex) {
      this.edge = Math.max(0, this.edge - 0.1 * dt);
      return 0;
    }

    /* Her excitement: more in the fast band (it is the more intense), a little when too slow, less than nothing when overwhelmed. */
    const intensity = lo > 0.4 ? 1.5 : 1;
    const comboBonus = 1 + this.comboBonus();
    let gain: number;

    switch (this.zone) {
      case 'good': gain = rate * intensity * comboBonus; break;
      case 'slow': gain = rate * 0.25 * (this.pace / lo); break;
      case 'over': gain = -rate * (0.3 + 0.7 * ((this.pace - hi) / Math.max(0.01, 1 - hi))); break;
      default: gain = -rate * 0.3;
    }

    /* The player's own: it climbs when the pace is high and cools when it is low. */
    const edgeMove = this.pace > 0.5 ? 0.16 * (this.pace - 0.5) / 0.5 : -0.09 * ((0.5 - this.pace) / 0.5);

    this.edge = Math.min(1, Math.max(0, this.edge + edgeMove * dt));

    return gain * dt;
  }

  /** How the scene ends if it is finished now, for the bar a fraction `fill` full: with her (the edge in its window), well, or too early. */
  ending(fill: number): Ending {
    if (fill < 1) return 'early';

    return this.edge >= 0.55 && this.edge < 1 ? 'together' : 'good';
  }
}
