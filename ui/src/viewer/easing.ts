/**
 * The easings a keyframe can name, as functions of how far along the segment it is (0 to 1). They mirror Strut's own `EasingType`, curve for curve, including where it differs
 * from the usual ones (its bounce is built from four parabolas and is the other way round from easings.net's; its step takes the value 0 to just under 1; its exponential does not
 * start at exactly 0), because what the viewer shows has to be what the game plays. A name is read without regard to capitals, as Strut reads it.
 */

type Ease = (t: number) => number;

const out = (f: Ease): Ease => t => 1 - f(1 - t);
const inOut = (f: Ease): Ease => t => (t < 0.5 ? f(t * 2) / 2 : 1 - f((1 - t) * 2) / 2);
const poly = (n: number): Ease => t => Math.pow(t, n);
const sine: Ease = t => 1 - Math.cos((t * Math.PI) / 2);
const expo: Ease = t => Math.pow(2, 10 * (t - 1));
const circ: Ease = t => 1 - Math.sqrt(1 - t * t);

/** Strut's back: the argument scales 1.70158 (1 when none is given). */
const back = (n: number | undefined): Ease => {
  const s = n === undefined ? 1.70158 : n * 1.70158;

  return t => t * t * ((s + 1) * t - s);
};

const elastic = (n: number | undefined): Ease => {
  const k = n ?? 1;

  return t => 1 - Math.pow(Math.cos((t * Math.PI) / 2), 3) * Math.cos(t * k * Math.PI);
};

/** Strut's bounce: the lowest of four parabolas, with the bounciness as the argument (0.5 when none is given). */
const bounce = (n: number | undefined): Ease => {
  const k = n ?? 0.5;

  return t => Math.min(
    Math.min((121 / 16) * t * t, (121 / 4) * k * Math.pow(t - 6 / 11, 2) + 1 - k),
    Math.min(121 * k * k * Math.pow(t - 9 / 11, 2) + 1 - k * k, 484 * k * k * k * Math.pow(t - 10.5 / 11, 2) + 1 - k * k * k)
  );
};

/** Strut's step: the argument is how many steps (2 when none is given), and the value goes 0, 1/n, ... up to (n-1)/n, so it only reaches the end value when the segment ends. */
function step(args: number[]): Ease {
  const steps = Math.max(2, Math.trunc(args[0] ?? 2));
  const length = 1 / steps;

  return t => (t < 0 ? 0 : Math.min(steps - 1, Math.floor(t / length + 1e-9)) * length);
}

/** `(t, ...arguments) => eased t`, by the name a keyframe gives. An unknown name is linear. */
export function ease(name: string | null, t: number, args: number[]): number {
  if (!name) return t;

  const lower = name.toLowerCase();

  if (lower === 'linear' || lower === 'none') return t;
  if (lower === 'step') return step(args)(t);

  const match = /^ease(in|out|inout)(quad|cubic|quart|quint|sine|expo|circ|back|elastic|bounce)$/.exec(lower);

  if (!match) return t;

  const [, direction, shape] = match;
  const base: Ease = {
    quad: poly(2), cubic: poly(3), quart: poly(4), quint: poly(5), sine, expo, circ,
    back: back(args[0]), elastic: elastic(args[0]), bounce: bounce(args[0])
  }[shape] as Ease;

  return direction === 'in' ? base(t) : direction === 'out' ? out(base)(t) : inOut(base)(t);
}

/** A point on the Catmull-Rom spline through `p1` and `p2`, with `p0` and `p3` as the points either side, `delta` of the way from `p1` to `p2`. */
export function splinePoint(delta: number, p0: number, p1: number, p2: number, p3: number): number {
  return 0.5 * (2 * p1 + (p2 - p0) * delta + (2 * p0 - 5 * p1 + 4 * p2 - p3) * delta * delta + (3 * p1 - p0 - 3 * p2 + p3) * delta * delta * delta);
}

const cubic = (s: number, p0: number, p1: number, p2: number, p3: number) => {
  const m = 1 - s;

  return m * m * m * p0 + 3 * m * m * s * p1 + 3 * m * s * s * p2 + s * s * s * p3;
};

const cubicSlope = (s: number, p0: number, p1: number, p2: number, p3: number) => {
  const m = 1 - s;

  return 3 * m * m * (p1 - p0) + 6 * m * s * (p2 - p1) + 3 * s * s * (p3 - p2);
};

/**
 * Blockbench's bezier segment, as Strut plays it: the start key's right handle and the end key's left handle are offsets from their keys, in the segment's own time and value. The
 * curve is not simply invertible in time, so the curve parameter whose time is the elapsed time is found by Newton-Raphson, and the value is read off that parameter.
 */
export function bezierPoint(elapsed: number, length: number, start: number, end: number, rightTime: number, rightValue: number, leftTime: number, leftValue: number): number {
  const t1 = Math.min(Math.max(rightTime, 0), length);
  const t2 = length + Math.min(Math.max(leftTime, -length), 0);
  let s = length > 1e-6 ? Math.min(Math.max(elapsed / length, 0), 1) : 0;

  for (let i = 0; i < 8; i++) {
    const slope = cubicSlope(s, 0, t1, t2, length);

    if (Math.abs(slope) < 1e-6) break;

    s = Math.min(Math.max(s - (cubic(s, 0, t1, t2, length) - elapsed) / slope, 0), 1);
  }

  return cubic(s, start, start + rightValue, end + leftValue, end);
}
