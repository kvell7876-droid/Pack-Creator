/**
 * The soft bones of a model, simulated the way the game simulates them (Strut's spring bone solver, ported): a physics file is read the same way (presets, keys, limits),
 * the entity's own motion pushes on each bone through the same filtered inertial force, and a damped spring pulls the bone back to its animated pose. Pure maths, no
 * three.js, so it is tested without a window.
 *
 * What it does not do: chains, cloth, flesh and the ragdoll (the game has them; the viewer only lists them), and collisions, so a bone's `squish` and the
 * layers it `collidesWith` have no effect here. Springs are driven by the motion of the whole model (see {@link Motion}), as in the game, where an entity standing
 * still moves its soft bones only through gravity.
 */

export type Vec3 = [number, number, number];

/** One soft bone's tuning, as the game reads it from a physics file. */
export interface SpringParams {
  /** Hertz: how fast it wobbles. */
  frequency: number;
  /** 0 rings forever, 1 returns without overshoot. */
  dampingRatio: number;
  /** How much of the model's own acceleration reaches the bone, 0 to 1. */
  inertia: number;
  /** Blocks per second squared, 0 for jiggle, about 9.8 for hair. */
  gravity: number;
  rotate: boolean;
  translate: boolean;
  /** The direction from the bone's pivot to the mass that lags, in the bone's own space (a unit vector). */
  tip: Vec3;
  /** The distance from the pivot to that mass, in blocks. */
  length: number;
  /** Per axis [min, max] in radians, or null for no limit on that axis. */
  rotationLimit: ([number, number] | null)[] | null;
  /** The most the bone may move, in model units (sixteenths of a block), or 0 for no limit. */
  translationLimit: number;
  translationScale: number;
  /** 0 to 1: how much of the physics reaches the pose. */
  weight: number;
}

/** One entry of the `soft` section: the game moves the meshes of these bones as a soft body (the viewer lists them, it does not simulate them). */
export interface SoftEntry {
  name: string;
  preset: string;
  bones: string[];
}

export interface ParsedPhysics {
  bones: Map<string, SpringParams>;
  soft: SoftEntry[];
  /** Parts of the file the game simulates and the viewer does not: `chain hair`, `cloth skirt`, `ragdoll`. */
  skipped: string[];
  warnings: string[];
}

/* =================================================================
 * Reading a physics file
 * ================================================================= */

const spring = (frequency: number, dampingRatio: number, gravity: number, tip: Vec3, length: number, translationLimit = 0): SpringParams => ({
  frequency, dampingRatio, inertia: 1, gravity, rotate: true, translate: false, tip, length, rotationLimit: null, translationLimit, translationScale: 1, weight: 1
});

const BASE = spring(2, 0.25, 0, [0, -1, 0], 0.15);

const PRESETS: Record<string, SpringParams> = {
  'jiggle-soft': spring(1.7, 0.24, 0, [0, -1, 0], 0.15),
  'jiggle-firm': spring(2.5, 0.32, 0, [0, -1, 0], 0.15),
  hair: spring(2.2, 0.3, 9.8, [0, -1, 0], 0.3),
  tail: spring(1.6, 0.2, 9.8, [0, -1, 0], 0.4),
  ear: spring(3.5, 0.2, 4, [0, 1, 0], 0.2),
  'soft-bone': spring(2.2, 0.28, 0, [0, -1, 0], 0.15, 1.5)
};

/** The keys the game reads that only matter to collisions, which the viewer does not do. */
const COLLISION_KEYS = new Set(['collidesWith', 'radius', 'squish', 'localRotation', 'collideBones', 'ignoreBones']);

const clamp01 = (value: number) => Math.max(0, Math.min(1, value));

function readLimit(object: Record<string, unknown>, where: string, warnings: string[]): ([number, number] | null)[] {
  return ['x', 'y', 'z'].map(axis => {
    const range = object[axis];

    if (!Array.isArray(range) || range.length < 2) return null;

    let lo = (Number(range[0]) * Math.PI) / 180;
    let hi = (Number(range[1]) * Math.PI) / 180;

    if (lo > hi) {
      warnings.push(`${where}: limit ${axis} has min above max; swapped`);
      [lo, hi] = [hi, lo];
    }

    return [lo, hi] as [number, number];
  });
}

/** Layers `object` over `start`, key by key, the way the game does: a preset first (it replaces every value), then the keys in order. */
function applyKeys(start: SpringParams, object: Record<string, unknown>, where: string, warnings: string[]): SpringParams {
  let next: SpringParams = { ...start, tip: [...start.tip] as Vec3 };

  if (typeof object.preset === 'string') {
    const preset = PRESETS[object.preset];

    if (preset) next = { ...preset, tip: [...preset.tip] as Vec3 };
    else warnings.push(`${where}: unknown preset "${object.preset}" (known: ${Object.keys(PRESETS).join(', ')})`);
  }

  for (const [key, value] of Object.entries(object)) {
    if (key === 'preset' || COLLISION_KEYS.has(key)) continue;

    try {
      switch (key) {
        case 'frequency': if (Number(value) > 0) next.frequency = Number(value); else warnings.push(`${where}: "frequency" must be above 0; kept ${next.frequency}`); break;
        case 'damping': next.dampingRatio = Math.max(0, Number(value)); break;
        case 'inertia': next.inertia = clamp01(Number(value)); break;
        case 'gravity': next.gravity = Number(value); break;
        case 'rotate': next.rotate = value === true; break;
        case 'translate': next.translate = value === true; break;
        case 'tip': {
          const [x, y, z] = (value as number[]).map(Number);
          const size = Math.hypot(x, y, z);

          if (size < 1e-4 || Number.isNaN(size)) warnings.push(`${where}: "tip" must not be the zero vector; kept ${next.tip.join(', ')}`);
          else next.tip = [x / size, y / size, z / size];
          break;
        }
        case 'length': if (Number(value) > 0) next.length = Number(value); else warnings.push(`${where}: "length" must be above 0; kept ${next.length}`); break;
        case 'limit': next.rotationLimit = readLimit(value as Record<string, unknown>, where, warnings); break;
        case 'maxTranslate': next.translationLimit = Math.max(0, Number(value)); break;
        case 'translateScale': next.translationScale = Number(value); break;
        case 'weight': next.weight = clamp01(Number(value)); break;
        default: warnings.push(`${where}: unknown key "${key}"`);
      }
    } catch {
      warnings.push(`${where}: "${key}" has a value that cannot be read`);
    }
  }

  return next;
}

/** Reads a model's physics file. A file that is not JSON gives no bones and says why. */
export function parsePhysics(text: string, source = 'physics file'): ParsedPhysics {
  const result: ParsedPhysics = { bones: new Map(), soft: [], skipped: [], warnings: [] };
  let root: Record<string, unknown>;

  try {
    root = JSON.parse(text) as Record<string, unknown>;
  } catch (error) {
    result.warnings.push(`${source}: not usable as physics JSON (${error instanceof Error ? error.message : String(error)})`);
    return result;
  }

  const defaults = (root.defaults ?? {}) as Record<string, unknown>;

  for (const [name, entry] of Object.entries((root.bones ?? {}) as Record<string, Record<string, unknown>>)) {
    const where = `${source} bone "${name}"`;

    result.bones.set(name, applyKeys(applyKeys(BASE, defaults, `${source} defaults`, result.warnings), entry, where, result.warnings));
  }

  for (const [name, entry] of Object.entries((root.flesh ?? root.soft ?? {}) as Record<string, { preset?: unknown; bones?: unknown }>)) {
    const bones = Array.isArray(entry.bones) ? entry.bones.filter((b): b is string => typeof b === 'string') : [];

    if (bones.length === 0) result.warnings.push(`${source} flesh "${name}": needs "bones", the bones of its region`);

    result.soft.push({ name, preset: typeof entry.preset === 'string' ? entry.preset : 'custom', bones });
  }

  for (const section of ['chains', 'cloth'] as const) {
    for (const name of Object.keys((root[section] ?? {}) as object)) result.skipped.push(`${section === 'chains' ? 'chain' : section} ${name}`);
  }

  if (root.ragdoll) result.skipped.push('ragdoll');

  return result;
}

/* =================================================================
 * The model's own motion
 * ================================================================= */

/** Where the model's origin is and which way it faces, over time; the springs feel its acceleration, not the place itself. */
interface Entity {
  position: Vec3;
  /** Degrees about the vertical axis. */
  yaw: number;
}

/** How the model is moved to wake its soft bones: what a girl walking, hopping or turning does to them, or nothing. */
export type Motion = 'still' | 'walk' | 'hop' | 'turn';

export const MOTIONS: { id: Motion; label: string }[] = [
  { id: 'still', label: 'Standing still' },
  { id: 'walk', label: 'Walking back and forth' },
  { id: 'hop', label: 'Hopping' },
  { id: 'turn', label: 'Turning on the spot' }
];

/** How long a bump lasts, in seconds. */
const BUMP_SECONDS = 0.25;

export function entityAt(motion: Motion, time: number, bumpAge: number): Entity {
  const entity: Entity = { position: [0, 0, 0], yaw: 0 };

  if (motion === 'walk') entity.position[2] = 0.8 * Math.sin((2 * Math.PI * time) / 3);
  else if (motion === 'hop') entity.position[1] = 0.3 * Math.max(0, Math.sin((2 * Math.PI * time) / 0.8));
  else if (motion === 'turn') entity.yaw = 50 * Math.sin((2 * Math.PI * time) / 2.4);

  if (bumpAge >= 0 && bumpAge < BUMP_SECONDS) entity.position[2] += 0.4 * Math.sin((Math.PI * bumpAge) / BUMP_SECONDS) ** 2;

  return entity;
}

/* =================================================================
 * Kinematics
 * ================================================================= */

const wrapDegrees = (degrees: number): number => {
  let wrapped = degrees % 360;

  if (wrapped >= 180) wrapped -= 360;
  if (wrapped < -180) wrapped += 360;

  return wrapped;
};

/**
 * What the model's motion does to a bone riding on it: the inertial force in a frame that moves and turns with it. Its velocity goes through two low-pass filters before
 * it is differentiated, as in the game, so the kinks of a path do not become spikes of force.
 */
export class Kinematics {
  /** The time constant of each of the two stages, in seconds. */
  static readonly SMOOTHING = 0.05;

  private started = false;
  private position: Vec3 = [0, 0, 0];
  private yaw = 0;
  private first: Vec3 = [0, 0, 0];
  private firstAngular = 0;
  private smooth: Vec3 = [0, 0, 0];
  private smoothAngular = 0;
  acceleration: Vec3 = [0, 0, 0];
  omega = 0;
  alpha = 0;

  reset(): void {
    this.started = false;
    this.first = [0, 0, 0];
    this.firstAngular = 0;
    this.smooth = [0, 0, 0];
    this.smoothAngular = 0;
    this.acceleration = [0, 0, 0];
    this.omega = 0;
    this.alpha = 0;
  }

  /** Takes the model's place at the end of a step of `h` seconds. The first call after a reset only records where it is. */
  advance(entity: Entity, h: number): void {
    const [x, y, z] = entity.position;

    if (!this.started) {
      this.position = [x, y, z];
      this.yaw = entity.yaw;
      this.started = true;
      return;
    }

    const velocity = [(x - this.position[0]) / h, (y - this.position[1]) / h, (z - this.position[2]) / h];
    const turn = (-wrapDegrees(entity.yaw - this.yaw) * Math.PI) / 180 / h;
    const k = 1 - Math.exp(-h / Kinematics.SMOOTHING);
    const next: Vec3 = [0, 0, 0];

    for (let i = 0; i < 3; i++) {
      this.first[i] += (velocity[i] - this.first[i]) * k;
      next[i] = this.smooth[i] + (this.first[i] - this.smooth[i]) * k;
      this.acceleration[i] = (next[i] - this.smooth[i]) / h;
    }

    this.firstAngular += (turn - this.firstAngular) * k;

    const nextAngular = this.smoothAngular + (this.firstAngular - this.smoothAngular) * k;

    this.alpha = (nextAngular - this.smoothAngular) / h;
    this.omega = nextAngular;
    this.smooth = next;
    this.smoothAngular = nextAngular;
    this.position = [x, y, z];
    this.yaw = entity.yaw;
  }

  /** The push on a mass at `lever` (its offset from the model's origin): minus the acceleration, the turn's Euler force and the centrifugal force, all times `inertia`. */
  inertialForce(lever: Vec3, inertia: number): Vec3 {
    const centrifugal = this.omega * this.omega;

    return [
      -inertia * (this.acceleration[0] + this.alpha * lever[2] - centrifugal * lever[0]),
      -inertia * this.acceleration[1],
      -inertia * (this.acceleration[2] - this.alpha * lever[0] - centrifugal * lever[2])
    ];
  }
}

/* =================================================================
 * The spring
 * ================================================================= */

/** The fixed step in seconds, and the most steps taken for one drawn frame (more time than that is dropped, so a hitch never makes a burst). */
export const STEP = 1 / 60;
export const MAX_STEPS = 6;

/** One bone's simulated state: rotation offset (radians) and translation offset (model units), each with its velocity. */
export class SpringState {
  rot: Vec3 = [0, 0, 0];
  rotVel: Vec3 = [0, 0, 0];
  pos: Vec3 = [0, 0, 0];
  posVel: Vec3 = [0, 0, 0];
  prevRot: Vec3 = [0, 0, 0];
  prevPos: Vec3 = [0, 0, 0];

  mark(): void {
    this.prevRot = [...this.rot];
    this.prevPos = [...this.pos];
  }

  reset(): void {
    this.rot = [0, 0, 0];
    this.rotVel = [0, 0, 0];
    this.pos = [0, 0, 0];
    this.posVel = [0, 0, 0];
    this.prevRot = [0, 0, 0];
    this.prevPos = [0, 0, 0];
  }
}

/**
 * Advances one bone by one step. `force` is the push on the mass in the bone's own space, in blocks per second squared. The spring is an XPBD constraint (a stiffness
 * that means the same at any step size and never blows up).
 */
export function stepSpring(state: SpringState, params: SpringParams, h: number, force: Vec3): void {
  const omega = 2 * Math.PI * params.frequency;
  const k = omega * omega;
  const keep = Math.exp(-2 * params.dampingRatio * omega * h);
  const stiffen = 1 / (1 + h * h * k);

  if (params.rotate) {
    /* The torque of a mass at tip * length under the force, over the moment of inertia (mass 1, radius length). */
    const [tx, ty, tz] = params.tip;
    const length = Math.max(1e-4, params.length);
    const torque = [(ty * force[2] - tz * force[1]) / length, (tz * force[0] - tx * force[2]) / length, (tx * force[1] - ty * force[0]) / length];

    for (let i = 0; i < 3; i++) {
      const velocity = (state.rotVel[i] + torque[i] * h) * keep;
      let solved = (state.rot[i] + velocity * h) * stiffen;
      const limit = params.rotationLimit?.[i];

      if (limit) solved = Math.max(limit[0], Math.min(limit[1], solved));

      state.rotVel[i] = (solved - state.rot[i]) / h;
      state.rot[i] = solved;
    }
  }

  if (params.translate) {
    for (let i = 0; i < 3; i++) {
      /* Blocks per second squared to model units per second squared. */
      const velocity = (state.posVel[i] + force[i] * 16 * h) * keep;
      let solved = (state.pos[i] + velocity * h) * stiffen;

      if (params.translationLimit > 0) solved = Math.max(-params.translationLimit, Math.min(params.translationLimit, solved));

      state.posVel[i] = (solved - state.pos[i]) / h;
      state.pos[i] = solved;
    }
  }
}

/* =================================================================
 * The simulation
 * ================================================================= */

/** Where a bone is in the model, from the animated pose: its pivot (blocks) and its three axes, each a unit vector in the model's space. */
export interface BoneFrame {
  origin: Vec3;
  axes: [Vec3, Vec3, Vec3];
}

/** What a soft bone adds to its animated pose right now: a rotation vector (radians, about the bone's own axes) and a move (model units). */
export interface BoneOffset {
  rot: Vec3;
  pos: Vec3;
}

export class SpringSim {
  private readonly states = new Map<string, SpringState>();
  private readonly kinematics = new Kinematics();
  private time = 0;
  private spare = 0;

  constructor(readonly params: Map<string, SpringParams>) {
    for (const name of params.keys()) this.states.set(name, new SpringState());
  }

  /** The simulation's own clock, in seconds since it was reset. */
  get now(): number {
    return this.time;
  }

  reset(): void {
    this.time = 0;
    this.spare = 0;
    this.kinematics.reset();

    for (const state of this.states.values()) state.reset();
  }

  /**
   * Moves time on by `dt` seconds in fixed steps. `entity` says where the model is at a moment, `frame` where a bone is (null for a bone the model does not have),
   * `wind` is a steady push in blocks per second squared.
   */
  advance(dt: number, entity: (time: number) => Entity, frame: (name: string) => BoneFrame | null, wind: Vec3): void {
    this.spare = Math.min(this.spare + dt, STEP * MAX_STEPS);

    const frames = new Map<string, BoneFrame | null>();

    while (this.spare >= STEP) {
      this.spare -= STEP;
      this.time += STEP;
      this.kinematics.advance(entity(this.time), STEP);

      for (const [name, params] of this.params) {
        const state = this.states.get(name)!;
        const where = frames.has(name) ? frames.get(name)! : frame(name);

        frames.set(name, where);

        if (!where || params.weight <= 0 || (!params.rotate && !params.translate)) {
          state.reset();
          continue;
        }

        const push = this.kinematics.inertialForce(where.origin, params.inertia);

        push[0] += wind[0];
        push[1] += wind[1] - params.gravity;
        push[2] += wind[2];

        /* Into the bone's own space: its axes are the rows of the way in. */
        const inBone: Vec3 = [0, 1, 2].map(i => where.axes[i][0] * push[0] + where.axes[i][1] * push[1] + where.axes[i][2] * push[2]) as Vec3;

        state.mark();
        stepSpring(state, params, STEP, inBone);
      }
    }
  }

  /** The offset of a bone, drawn between the states before and after the last step so it moves smoothly, times the bone's weight. */
  offset(name: string): BoneOffset | null {
    const params = this.params.get(name);
    const state = this.states.get(name);

    if (!params || !state || params.weight <= 0) return null;

    const t = this.spare / STEP;
    const between = (before: Vec3, after: Vec3, scale: number): Vec3 => [0, 1, 2].map(i => (before[i] + (after[i] - before[i]) * t) * scale) as Vec3;

    return {
      rot: params.rotate ? between(state.prevRot, state.rot, params.weight) : [0, 0, 0],
      pos: params.translate ? between(state.prevPos, state.pos, params.translationScale * params.weight) : [0, 0, 0]
    };
  }
}
