import type { Role } from './roles';

/*
 * The game's scene rules, ported as they are: `ScenePhase`, `SceneRules` and `ScenePlanner` (entity/scene/session). The simulator runs the real decisions, so what it shows is
 * what the game would do with the same clips and cues. Keep this in step with the Java.
 */

export type Phase = 'NONE' | 'DIALOG' | 'LAYING_DOWN' | 'BED_IDLE' | 'INTRO' | 'MOVING' | 'HAVING_SEX' | 'CUM' | 'STATIONARY_INTRO' | 'STATIONARY';

export const showsSceneRig = (phase: Phase): boolean => !(phase === 'NONE' || phase === 'BED_IDLE' || phase === 'LAYING_DOWN');
export const isSequence = (phase: Phase): boolean => phase === 'INTRO' || phase === 'STATIONARY_INTRO';
export const nextAfterSequence = (phase: Phase): Phase => (phase === 'INTRO' ? 'HAVING_SEX' : phase === 'STATIONARY_INTRO' ? 'STATIONARY' : phase);

export type LoopMode = 'LOOP' | 'HOLD';

/** What a scene is, at this moment: the state `SceneState` holds. */
export class SceneStateData {
  inScene = false;
  phase: Phase = 'NONE';
  sexAnim: string | null = null;
  thrusting = false;
  introIndex = 0;
  stationaryIndex = 0;
  progress = 0;
  cumThreshold = 0;
  stationaryLoop = 0;
  stationaryLoopThreshold = 0;
}

export interface Hooks {
  cumFinished(): void;
  soloFinished(): void;
}

// =================================================================
// Scene rules
// =================================================================

export function playPhase(state: SceneStateData, phase: Phase): void {
  state.phase = phase;

  if (phase !== 'INTRO') state.introIndex = 0;
}

export function readyToCum(state: SceneStateData): boolean {
  return state.inScene && state.progress >= state.cumThreshold && state.phase !== 'CUM';
}

/** A thrust cue adds to the progress while she is having sex; the progress never leaves 0 to the threshold. */
export function thrusted(state: SceneStateData, cue: string, step: number): void {
  if (state.phase !== 'HAVING_SEX') return;

  if (cue.includes('thrust')) state.progress += step;

  state.progress = Math.min(Math.max(state.progress, 0), state.cumThreshold);
}

export function clipFinished(state: SceneStateData, endOfSequence: boolean, hooks: Hooks): void {
  if (!state.inScene) return;

  switch (state.phase) {
    case 'INTRO':
    case 'STATIONARY_INTRO':
      if (endOfSequence) playPhase(state, nextAfterSequence(state.phase));
      else if (state.phase === 'INTRO') state.introIndex += 1;
      else state.stationaryIndex += 1;
      break;
    case 'CUM':
      hooks.cumFinished();
      break;
    case 'LAYING_DOWN':
      playPhase(state, 'BED_IDLE');
      break;
    case 'STATIONARY':
      if (state.stationaryLoop < state.stationaryLoopThreshold) state.stationaryLoop += 1;
      else hooks.soloFinished();
      break;
    default:
      break;
  }
}

export function clearScene(state: SceneStateData): void {
  state.introIndex = 0;
  state.stationaryIndex = 0;
  state.progress = 0;
  state.phase = 'NONE';
  state.inScene = false;
}

// =================================================================
// The planner
// =================================================================

export const SLOW_REPEATS_PER_SIDE = 2;
/** A backstop for a side whose cue never arrives, in ticks. */
export const SIDED_STEP_TIMEOUT_TICKS = 200;

const NOT_STARTED = Number.MIN_SAFE_INTEGER;

type SidedStep = 'LEFT' | 'LEFT_SWAP' | 'RIGHT' | 'RIGHT_SWAP';

export interface Clips {
  of(role: Role): string[];
  /** The clip of a role, or the one that stands for none when the scene declares nothing. */
  orNone(role: Role): string;
  startingSide(): Role;
}

export interface Snapshot {
  phase: Phase;
  introIndex: number;
  stationaryIndex: number;
  stationaryLoop: number;
  stationaryLoopThreshold: number;
  thrusting: boolean;
  sexAnim: string | null;
  useKeyFrameEvents: boolean;
}

export interface Choice {
  clip: string;
  loop: LoopMode;
}

export interface Plan {
  choice: Choice | null;
  restart: boolean;
  sexAnim: string | null;
  sexAnimChanged: boolean;
  moveTo: Phase | null;
  stop: boolean;
}

const play = (choice: Choice | null, restart: boolean): Plan => ({ choice, restart, sexAnim: null, sexAnimChanged: false, moveTo: null, stop: false });

export const snapshotsEqual = (a: Snapshot | null, b: Snapshot): boolean =>
  !!a && a.phase === b.phase && a.introIndex === b.introIndex && a.stationaryIndex === b.stationaryIndex && a.stationaryLoop === b.stationaryLoop && a.stationaryLoopThreshold === b.stationaryLoopThreshold
  && a.thrusting === b.thrusting && a.sexAnim === b.sexAnim && a.useKeyFrameEvents === b.useKeyFrameEvents;

export class ScenePlanner {
  private sidedStep: SidedStep = 'LEFT';
  private sidedRepeats = 0;
  private sidedStepStartedAt = NOT_STARTED;
  private thrustingSince = NOT_STARTED;
  private lastPhase: Phase | null = null;
  private lastIntroIndex = -1;
  private lastStationaryIndex = -1;

  constructor(private readonly random: () => number = Math.random) {}

  private resetSidedCycle(): void {
    this.sidedStep = 'LEFT';
    this.sidedRepeats = 0;
    this.sidedStepStartedAt = NOT_STARTED;
    this.thrustingSince = NOT_STARTED;
  }

  /** Whether the cycle is waiting out its backstop at this age: it must be asked again then even if nothing else happened. */
  timeoutDue(s: Snapshot, age: number): boolean {
    return s.phase === 'HAVING_SEX' && !s.thrusting && this.sidedStepStartedAt !== NOT_STARTED && age - this.sidedStepStartedAt >= SIDED_STEP_TIMEOUT_TICKS;
  }

  plan(s: Snapshot, clips: Clips, cues: Set<string>, age: number, stepEnded: boolean): Plan {
    const moved = s.phase !== this.lastPhase || s.introIndex !== this.lastIntroIndex || s.stationaryIndex !== this.lastStationaryIndex;

    if (moved) {
      this.lastPhase = s.phase;
      this.lastIntroIndex = s.introIndex;
      this.lastStationaryIndex = s.stationaryIndex;
      this.resetSidedCycle();
    }

    switch (s.phase) {
      case 'LAYING_DOWN':
        return play({ clip: clips.orNone('lay_on_bed'), loop: 'HOLD' }, moved);
      case 'BED_IDLE':
        return play({ clip: clips.orNone('bed_idle'), loop: 'LOOP' }, moved);
      case 'INTRO':
      case 'STATIONARY_INTRO': {
        const sequence = clips.of(s.phase === 'INTRO' ? 'intro' : 'solo_intro');

        if (sequence.length === 0) return { choice: null, restart: false, sexAnim: null, sexAnimChanged: false, moveTo: nextAfterSequence(s.phase), stop: false };

        const index = s.phase === 'INTRO' ? s.introIndex : s.stationaryIndex;

        return play({ clip: sequence[Math.min(index, sequence.length - 1)], loop: 'HOLD' }, moved);
      }
      case 'HAVING_SEX':
        return this.sex(s, clips, cues, age, stepEnded, moved);
      case 'CUM':
        return { choice: { clip: clips.orNone('cum'), loop: 'HOLD' }, restart: moved, sexAnim: null, sexAnimChanged: true, moveTo: null, stop: false };
      case 'STATIONARY': {
        const loops = clips.of('loop');

        if (loops.length === 0 || s.stationaryLoop >= s.stationaryLoopThreshold) return { choice: null, restart: false, sexAnim: null, sexAnimChanged: false, moveTo: null, stop: true };

        return play({ clip: loops[0], loop: 'HOLD' }, moved);
      }
      default:
        return play(null, false);
    }
  }

  private sex(s: Snapshot, clips: Clips, cues: Set<string>, age: number, stepEnded: boolean, moved: boolean): Plan {
    const thrusting = s.thrusting;
    const slow = clips.of('slow');
    const fast = clips.of('fast');
    const switchFired = cues.has('switch');
    const resetFired = cues.has('reset');
    const readyFired = cues.has('ready') || cues.has('rapid');

    /* The cycle's clock is held while he is thrusting, so coming back resumes the side she was on: the time spent thrusting is added back when it stops. */
    if (thrusting && this.thrustingSince === NOT_STARTED) this.thrustingSince = age;

    if (!thrusting && this.thrustingSince !== NOT_STARTED) {
      if (this.sidedStepStartedAt !== NOT_STARTED) this.sidedStepStartedAt += age - this.thrustingSince;

      this.thrustingSince = NOT_STARTED;
    }

    const sided = thrusting ? null : this.sidedSlow(clips, cues.has('swap'), stepEnded, age);

    if (sided) return play(sided, moved);

    let sexAnim = s.sexAnim;
    let changed = false;

    if (sexAnim === null) {
      sexAnim = this.pick(slow);
      changed = true;
    }

    if (s.useKeyFrameEvents) {
      if (switchFired && thrusting) {
        sexAnim = this.pick(fast);
        changed = true;
      }

      if (resetFired && thrusting) {
        const chosen = this.pick(fast);

        return chosen === null ? { choice: null, restart: false, sexAnim: null, sexAnimChanged: changed, moveTo: null, stop: false } : { choice: { clip: chosen, loop: 'LOOP' }, restart: true, sexAnim: chosen, sexAnimChanged: true, moveTo: null, stop: false };
      }

      if (resetFired) {
        sexAnim = this.pick(slow);
        changed = true;
      }

      return { choice: sexAnim === null ? null : { clip: sexAnim, loop: 'LOOP' }, restart: moved, sexAnim, sexAnimChanged: changed, moveTo: null, stop: false };
    }

    if ((resetFired || readyFired) && thrusting) {
      const chosen = this.pick(fast);

      return chosen === null ? { choice: null, restart: false, sexAnim: null, sexAnimChanged: changed, moveTo: null, stop: false } : { choice: { clip: chosen, loop: 'LOOP' }, restart: true, sexAnim: chosen, sexAnimChanged: true, moveTo: null, stop: false };
    }

    const anims = thrusting ? fast : slow;

    if (anims.length === 0) return { choice: null, restart: false, sexAnim, sexAnimChanged: changed, moveTo: null, stop: false };

    /* The fast clip a restart chose stays until the next restart. */
    const chosen = thrusting && sexAnim !== null && fast.includes(sexAnim) ? sexAnim : anims[0];

    return { choice: { clip: chosen, loop: 'LOOP' }, restart: moved, sexAnim, sexAnimChanged: changed, moveTo: null, stop: false };
  }

  /** The one clip the left and right side cycle wants now, or null for a scene without both sides. */
  private sidedSlow(clips: Clips, swapCue: boolean, stepEnded: boolean, age: number): Choice | null {
    const left = clips.of('slow_L')[0] ?? null;
    const right = clips.of('slow_R')[0] ?? null;

    if (left === null || right === null) return null;

    const leftSwap = clips.of('slow_L_swap')[0] ?? null;
    const rightSwap = clips.of('slow_R_swap')[0] ?? null;

    if (this.sidedStepStartedAt === NOT_STARTED) this.beginSidedStep(clips.startingSide() === 'slow_R' ? 'RIGHT' : 'LEFT', age);

    this.advanceSidedStep(leftSwap, rightSwap, swapCue, stepEnded, age);

    let clip: string | null = this.sidedStep === 'LEFT' ? left : this.sidedStep === 'RIGHT' ? right : this.sidedStep === 'LEFT_SWAP' ? leftSwap : rightSwap;

    if (clip === null) {
      clip = this.sidedStep === 'LEFT_SWAP' ? right : left;
      this.beginSidedStep(this.sidedStep === 'LEFT_SWAP' ? 'RIGHT' : 'LEFT', age);
    }

    return { clip, loop: this.sidedStep === 'LEFT' || this.sidedStep === 'RIGHT' ? 'LOOP' : 'HOLD' };
  }

  private advanceSidedStep(leftSwap: string | null, rightSwap: string | null, swapCue: boolean, stepEnded: boolean, age: number): void {
    const timedOut = age - this.sidedStepStartedAt >= SIDED_STEP_TIMEOUT_TICKS;

    if (this.sidedStep === 'LEFT' || this.sidedStep === 'RIGHT') {
      if (swapCue) this.sidedRepeats++;

      if (!swapCue && !timedOut) return;
      if (this.sidedRepeats < SLOW_REPEATS_PER_SIDE && !timedOut) return;

      const onLeft = this.sidedStep === 'LEFT';
      const swap = onLeft ? leftSwap : rightSwap;

      if (swap !== null) this.beginSidedStep(onLeft ? 'LEFT_SWAP' : 'RIGHT_SWAP', age);
      else this.beginSidedStep(onLeft ? 'RIGHT' : 'LEFT', age);
    } else {
      if (!stepEnded && !timedOut) return;

      this.beginSidedStep(this.sidedStep === 'LEFT_SWAP' ? 'RIGHT' : 'LEFT', age);
    }
  }

  private beginSidedStep(step: SidedStep, age: number): void {
    this.sidedStep = step;
    this.sidedRepeats = 0;
    this.sidedStepStartedAt = age;
  }

  private pick(list: string[]): string | null {
    if (list.length === 0) return null;

    return list.length === 1 ? list[0] : list[Math.floor(this.random() * list.length)];
  }
}
