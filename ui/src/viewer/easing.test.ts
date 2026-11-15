/*
 * The viewer's easings, held to Strut's own EasingType (worked out by hand from its source). Run with `npm run test:easing`.
 */
import assert from 'node:assert/strict';
import { bezierPoint, ease, splinePoint } from './easing';

const near = (actual: number, expected: number, message = '') => assert.ok(Math.abs(actual - expected) < 1e-6, `${message} ${actual} is not ${expected}`);

/* Names are read without regard to capitals, and none/linear/unknown are linear. */
near(ease('EASEINSINE', 0.5, []), 1 - Math.cos(Math.PI / 4));
near(ease('none', 0.3, []), 0.3);
near(ease('nonsense', 0.3, []), 0.3);

/* The plain curves. */
near(ease('easeInQuad', 0.5, []), 0.25);
near(ease('easeOutQuad', 0.5, []), 0.75);
near(ease('easeInOutCubic', 0.25, []), 0.0625);
near(ease('easeInExpo', 0, []), Math.pow(2, -10), 'Strut does not start the exponential at exactly 0');

/* Strut's bounce is four parabolas with bounciness 0.5 by default, and "in" is the curve itself. At 0.5: parabola one is 1.890625, two is 0.5 + 121/4*0.5*(0.5-6/11)^2. */
const two = (121 / 4) * 0.5 * Math.pow(0.5 - 6 / 11, 2) + 0.5;

near(ease('easeInBounce', 0.5, []), Math.min(1.890625, two, 121 * 0.25 * Math.pow(0.5 - 9 / 11, 2) + 0.75, 484 * 0.125 * Math.pow(0.5 - 10.5 / 11, 2) + 0.875));
near(ease('easeInBounce', 1, []), 1);
near(ease('easeOutBounce', 0, []), 0);

/* Back and elastic take the argument; none means 1. */
near(ease('easeInBack', 0.5, []), 0.25 * ((1.70158 + 1) * 0.5 - 1.70158));
near(ease('easeInBack', 0.5, [2]), 0.25 * ((3.40316 + 1) * 0.5 - 3.40316));
near(ease('easeInElastic', 0.5, [2]), 1 - Math.pow(Math.cos(Math.PI / 4), 3) * Math.cos(0.5 * 2 * Math.PI));

/* Step goes 0, 1/n, ... (n-1)/n and never reaches 1 inside the segment; two steps by default. */
near(ease('step', 0.4, []), 0);
near(ease('step', 0.6, []), 0.5);
near(ease('step', 0.99, [4]), 0.75);
near(ease('step', 0.3, [4]), 0.25);

/* The Catmull-Rom spline passes through the two ends of its segment, and is straight when the guides are on the line. */
near(splinePoint(0, 5, 10, 20, 30), 10);
near(splinePoint(1, 5, 10, 20, 30), 20);
near(splinePoint(0.5, 0, 10, 20, 30), 15);

/* A bezier whose handles lie on the straight line is the straight line (a segment 2 s long going from 0 to 10, handles a third of the way). */
near(bezierPoint(1, 2, 0, 10, 2 / 3, 10 / 3, -2 / 3, -10 / 3), 5);
near(bezierPoint(0.5, 2, 0, 10, 2 / 3, 10 / 3, -2 / 3, -10 / 3), 2.5);
near(bezierPoint(2, 2, 0, 10, 0, 0, 0, 0), 10);

console.log('easing ok');
