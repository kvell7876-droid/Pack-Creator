/*
 * The spring bone simulation, held to what the game's solver does. Run with `npm run test:physics`: there is no test framework, only assertions.
 */
import assert from 'node:assert/strict';
import { entityAt, Kinematics, parsePhysics, SpringSim, SpringState, STEP, stepSpring, type BoneFrame, type SpringParams, type Vec3 } from './physics';

const near = (actual: number, expected: number, tolerance = 1e-6) => assert.ok(Math.abs(actual - expected) <= tolerance, `${actual} is not ${expected}`);
const upright: BoneFrame = { origin: [0, 1, 0], axes: [[1, 0, 0], [0, 1, 0], [0, 0, 1]] };

/* A preset sets the starting values, whatever order the keys come in, and a key after it wins. */
{
  const parsed = parsePhysics('{"bones":{"a":{"frequency":3,"preset":"hair"},"b":{"preset":"soft-bone","damping":0.5}}}');
  const a = parsed.bones.get('a')!;
  const b = parsed.bones.get('b')!;

  near(a.frequency, 3);
  near(a.gravity, 9.8);
  near(a.length, 0.3);
  near(b.frequency, 2.2);
  near(b.dampingRatio, 0.5);
  near(b.translationLimit, 1.5);
  assert.equal(parsed.warnings.length, 0);
}

/* Defaults apply to every bone; limits are read in degrees and kept in radians; an axis with no range has no limit. */
{
  const parsed = parsePhysics('{"defaults":{"inertia":0.5},"bones":{"a":{"limit":{"x":[-30,30],"y":[10,-10]}}}}');
  const a = parsed.bones.get('a')!;

  near(a.inertia, 0.5);
  near(a.rotationLimit![0]![1], (30 * Math.PI) / 180);
  near(a.rotationLimit![1]![0], (-10 * Math.PI) / 180);
  assert.equal(a.rotationLimit![2], null);
  assert.ok(parsed.warnings.some(w => w.includes('swapped')));
}

/* What the game accepts but the viewer ignores (collisions) is silent; a key nobody knows is reported; the parts it does not simulate are listed. */
{
  const parsed = parsePhysics('{"bones":{"a":{"collidesWith":["players"],"radius":0.1,"squish":0.2,"wobble":1}},"chains":{"hair":{"bones":["h1"]}},"cloth":{"skirt":{"bone":"s"}},"ragdoll":{}}');

  assert.deepEqual(parsed.warnings.length, 1);
  assert.ok(parsed.warnings[0].includes('wobble'));
  assert.deepEqual(parsed.skipped, ['chain hair', 'cloth skirt', 'ragdoll']);
  assert.equal(parsePhysics('not json').bones.size, 0);
  assert.equal(parsePhysics('not json').warnings.length, 1);
}

/* Soft bodies are listed with their bones (the game moves them; the viewer does not simulate them), and a soft entry with no bones is reported. */
{
  const parsed = parsePhysics('{"bones":{},"flesh":{"breasts":{"preset":"pouch","bones":["boobL","boobR"]},"belly":{"preset":"belly"}}}');

  assert.deepEqual(parsed.soft.map(e => [e.name, e.preset, e.bones]), [['breasts', 'pouch', ['boobL', 'boobR']], ['belly', 'belly', []]]);
  assert.ok(parsed.warnings.some(w => w.includes('belly') && w.includes('bones')));
  assert.equal(parsed.bones.size, 0);
}

/* A bone at rest with no force stays at rest. */
{
  const params = parsePhysics('{"bones":{"a":{"preset":"jiggle-soft"}}}').bones.get('a')!;
  const state = new SpringState();

  for (let i = 0; i < 600; i++) stepSpring(state, params, STEP, [0, 0, 0]);

  assert.deepEqual(state.rot, [0, 0, 0]);
}

/* A steady push sideways bends the bone to where the spring balances it, and a push the other way bends it the other way. */
{
  const params = parsePhysics('{"bones":{"a":{"frequency":2,"damping":0.25,"length":0.15}}}').bones.get('a')!;
  const push = (force: Vec3) => {
    const state = new SpringState();

    for (let i = 0; i < 1200; i++) stepSpring(state, params, STEP, force);

    return state.rot;
  };
  const k = (2 * Math.PI * 2) ** 2;
  const keep = Math.exp(-2 * 0.25 * 2 * Math.PI * 2 * STEP);
  /* tip (0,-1,0) under a push along +X turns about +Z by push / length. */
  const expected = (keep * 9.8) / 0.15 / k;

  near(push([9.8, 0, 0])[2], expected, 1e-4);
  near(push([-9.8, 0, 0])[2], -expected, 1e-4);
  near(push([9.8, 0, 0])[0], 0, 1e-9);
}

/* A limit holds a bone however hard it is pushed. */
{
  const params = parsePhysics('{"bones":{"a":{"limit":{"z":[-20,20]}}}}').bones.get('a')!;
  const state = new SpringState();

  for (let i = 0; i < 600; i++) stepSpring(state, params, STEP, [500, 0, 0]);

  near(state.rot[2], (20 * Math.PI) / 180);
}

/* A bone with translate moves, and stops at its limit in model units. */
{
  const params = parsePhysics('{"bones":{"a":{"rotate":false,"translate":true,"maxTranslate":1.5}}}').bones.get('a')!;
  const state = new SpringState();

  for (let i = 0; i < 600; i++) stepSpring(state, params, STEP, [0, 50, 0]);

  near(state.pos[1], 1.5);
  near(state.rot[0], 0);
}

/* A model moving at a steady speed pushes on nothing; one speeding up pushes back the other way. */
{
  const kinematics = new Kinematics();
  let z = 0;

  for (let i = 0; i < 120; i++) { z += 1 * STEP; kinematics.advance({ position: [0, 0, z], yaw: 0 }, STEP); }

  near(kinematics.inertialForce([0, 1, 0], 1)[2], 0, 1e-3);

  for (let i = 0; i < 5; i++) { z += (1 + (i + 1) * 0.2) * STEP; kinematics.advance({ position: [0, 0, z], yaw: 0 }, STEP); }

  assert.ok(kinematics.inertialForce([0, 1, 0], 1)[2] < -0.1, 'speeding up along +Z pushes the bone toward -Z');
  near(kinematics.inertialForce([0, 1, 0], 0)[2], 0);
}

/* Standing still wakes nothing in a jiggle bone; walking does; hair hangs without moving when it is straight. */
{
  const parsed = parsePhysics('{"bones":{"jiggle":{"preset":"jiggle-soft"},"hair":{"preset":"hair"}}}');
  const frame = (): BoneFrame => upright;
  const run = (motion: 'still' | 'walk') => {
    const sim = new SpringSim(parsed.bones);

    for (let i = 0; i < 300; i++) sim.advance(STEP, time => entityAt(motion, time, -1), frame, [0, 0, 0]);

    return sim;
  };
  const size = (sim: SpringSim, name: string) => Math.hypot(...sim.offset(name)!.rot);

  assert.ok(size(run('still'), 'jiggle') < 1e-9);
  assert.ok(size(run('still'), 'hair') < 1e-9);
  assert.ok(size(run('walk'), 'jiggle') > 0.001);
}

/* A bone the model does not have is skipped, and a weight of 0 gives the animation back exactly. */
{
  const parsed = parsePhysics('{"bones":{"missing":{},"off":{"weight":0}}}');
  const sim = new SpringSim(parsed.bones);

  sim.advance(0.5, time => entityAt('walk', time, -1), name => (name === 'off' ? upright : null), [0, 0, 0]);

  assert.equal(sim.offset('off'), null);
  assert.deepEqual(sim.offset('missing')!.rot, [0, 0, 0]);
}

/* Wind pushes a bone, and a bone's own axes decide which way: turned a quarter about Y, the same push bends it about the other axis. */
{
  const params = new Map<string, SpringParams>(parsePhysics('{"bones":{"a":{}}}').bones);
  const turned: BoneFrame = { origin: [0, 1, 0], axes: [[0, 0, -1], [0, 1, 0], [1, 0, 0]] };
  const bend = (frame: BoneFrame) => {
    const sim = new SpringSim(params);

    for (let i = 0; i < 300; i++) sim.advance(STEP, () => entityAt('still', 0, -1), () => frame, [5, 0, 0]);

    return sim.offset('a')!.rot;
  };

  assert.ok(Math.abs(bend(upright)[2]) > 0.05);
  near(bend(upright)[0], 0, 1e-9);
  assert.ok(Math.abs(bend(turned)[0]) > 0.05);
  near(bend(turned)[2], 0, 1e-9);
}

console.log('physics: all passed');
