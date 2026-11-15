/*
 * The planner and the rules of the simulator, held to the cases of the game's own `ScenePlannerTest`. Run with `npm run test:sim` (it bundles this file with esbuild and runs it
 * in Node): there is no test framework, only assertions, and the process fails on the first one that does not hold.
 */
import assert from 'node:assert/strict';
import { ScenePlanner, SceneStateData, clipFinished, playPhase, readyToCum, thrusted, type Clips, type Phase, type Snapshot } from './planner';
import { deriveFeel } from './feel';
import { SceneSim, type SceneInfo, type Stage } from './engine';
import { DEFAULT_PACE, PaceGame } from './pace';
import { SceneCatalogue, parseClipName, type Role } from './roles';

class TestClips implements Clips {
  clips = new Map<Role, string[]>();
  opening: Role = 'slow_L';

  with(role: Role, ...names: string[]): this {
    this.clips.set(role, names);
    return this;
  }

  of(role: Role): string[] { return this.clips.get(role) ?? []; }
  orNone(role: Role): string { return this.of(role)[0] ?? 'none'; }
  startingSide(): Role { return this.opening; }
}

const at = (phase: Phase): Snapshot => ({ phase, introIndex: 0, stationaryIndex: 0, stationaryLoop: 0, stationaryLoopThreshold: 0, thrusting: false, sexAnim: null, useKeyFrameEvents: false });
const sex = (thrusting: boolean, sexAnim: string | null, keyFrames: boolean): Snapshot => ({ ...at('HAVING_SEX'), thrusting, sexAnim, useKeyFrameEvents: keyFrames });
const none = new Set<string>();
const seeded = (): (() => number) => { let n = 1; return () => { n = (n * 16807) % 2147483647; return n / 2147483647; }; };

// --- the phases that are one clip ---

{
  const clips = new TestClips().with('lay_on_bed', 'lay').with('bed_idle', 'idle');
  const planner = new ScenePlanner(seeded());
  const lay = planner.plan(at('LAYING_DOWN'), clips, none, 0, false);

  assert.deepEqual(lay.choice, { clip: 'lay', loop: 'HOLD' });
  assert.equal(lay.restart, true, 'the first time in a phase it starts the clip');
  assert.equal(planner.plan(at('LAYING_DOWN'), clips, none, 1, false).restart, false, 'asked again with nothing changed it does not restart');

  const idle = planner.plan(at('BED_IDLE'), clips, none, 2, true);

  assert.deepEqual(idle.choice, { clip: 'idle', loop: 'LOOP' });
  assert.equal(idle.restart, true);
}

assert.deepEqual(new ScenePlanner().plan(at('LAYING_DOWN'), new TestClips(), none, 0, false).choice, { clip: 'none', loop: 'HOLD' }, 'no clip plays the none clip');

{
  const clips = new TestClips().with('intro', 'a', 'b', 'c');
  const planner = new ScenePlanner();
  const intro = (index: number): Snapshot => ({ ...at('INTRO'), introIndex: index });

  assert.equal(planner.plan(intro(0), clips, none, 0, false).choice?.clip, 'a');

  const second = planner.plan(intro(1), clips, none, 1, true);

  assert.equal(second.choice?.clip, 'b');
  assert.equal(second.restart, true, 'the next clip of a sequence is a new start');
  assert.equal(planner.plan(intro(9), clips, none, 2, true).choice?.clip, 'c', 'past the end keeps the last clip');
}

{
  const plan = new ScenePlanner().plan(at('INTRO'), new TestClips(), none, 0, false);

  assert.equal(plan.choice, null);
  assert.equal(plan.moveTo, 'HAVING_SEX', 'an intro with no clips moves on');
}

{
  const clips = new TestClips().with('loop', 'loop');
  const planner = new ScenePlanner();
  const solo = (loops: number): Snapshot => ({ ...at('STATIONARY'), stationaryLoop: loops, stationaryLoopThreshold: 3 });

  assert.equal(planner.plan(solo(1), clips, none, 0, false).choice?.clip, 'loop');
  assert.equal(planner.plan(solo(3), clips, none, 1, true).stop, true);
  assert.equal(planner.plan(solo(0), new TestClips(), none, 2, true).stop, true, 'no loop clip, nothing to play');
}

{
  const plan = new ScenePlanner().plan(at('CUM'), new TestClips().with('cum', 'cum'), none, 0, false);

  assert.equal(plan.choice?.clip, 'cum');
  assert.equal(plan.sexAnimChanged, true);
  assert.equal(plan.sexAnim, null);
}

// --- the sex phase ---

{
  const clips = new TestClips().with('slow', 'slow').with('fast', 'fast', 'fast_2');
  const planner = new ScenePlanner(seeded());
  const idle = planner.plan(sex(false, null, false), clips, none, 0, false);

  assert.deepEqual(idle.choice, { clip: 'slow', loop: 'LOOP' });
  assert.equal(idle.sexAnim, 'slow', 'the scene remembers the sex clip it picked');

  const thrust = planner.plan(sex(true, 'slow', false), clips, none, 1, false);

  assert.equal(thrust.choice?.clip, 'fast', 'thrusting plays the first fast clip');

  const restarted = planner.plan(sex(true, 'slow', false), clips, new Set(['reset']), 2, false);

  assert.equal(restarted.restart, true, 'a reset cue while thrusting restarts a fast clip');
  assert.ok(['fast', 'fast_2'].includes(restarted.choice!.clip));
}

{
  /* With keyframe events the clip changes on the cue, not on the thrusting. */
  const clips = new TestClips().with('slow', 'slow').with('fast', 'fast');
  const planner = new ScenePlanner(seeded());

  assert.equal(planner.plan(sex(true, 'slow', true), clips, none, 0, false).choice?.clip, 'slow', 'thrusting alone changes nothing');
  assert.equal(planner.plan(sex(true, 'slow', true), clips, new Set(['switch']), 1, false).choice?.clip, 'fast');
  assert.equal(planner.plan(sex(true, 'fast', true), clips, new Set(['reset']), 2, false).restart, true);
  assert.equal(planner.plan(sex(false, 'fast', true), clips, new Set(['reset']), 3, false).choice?.clip, 'slow', 'a reset when not thrusting goes back to slow');
}

{
  /* The left and right cycle: two repeats a side, then the swap clip, then the other side. */
  const clips = new TestClips().with('slow_L', 'L').with('slow_R', 'R').with('slow_L_swap', 'LtoR').with('slow_R_swap', 'RtoL');
  const planner = new ScenePlanner();
  const ask = (cues: string[], age: number, ended = false) => planner.plan(sex(false, null, false), clips, new Set(cues), age, ended).choice;

  assert.deepEqual(ask([], 0), { clip: 'L', loop: 'LOOP' }, 'opens on the starting side');
  assert.equal(ask(['swap'], 1)?.clip, 'L', 'one swap cue is not enough');
  assert.deepEqual(ask(['swap'], 2), { clip: 'LtoR', loop: 'HOLD' }, 'the second moves her across');
  assert.deepEqual(ask([], 3, true), { clip: 'R', loop: 'LOOP' }, 'and when the move has played she works the other side');
}

assert.deepEqual(new ScenePlanner().plan(sex(false, null, false), new TestClips().with('slow_L', 'L').with('slow_R', 'R'), none, 0, false).choice?.clip, 'L');

// --- the rules ---

{
  const state = new SceneStateData();

  state.inScene = true;
  state.cumThreshold = 1;
  playPhase(state, 'HAVING_SEX');
  thrusted(state, 'pound-thrust', 0.4);
  assert.ok(Math.abs(state.progress - 0.4) < 1e-9);
  thrusted(state, 'moan', 0.4);
  assert.ok(Math.abs(state.progress - 0.4) < 1e-9, 'only a thrust cue adds');
  thrusted(state, 'thrust', 5);
  assert.equal(state.progress, 1, 'never past the threshold');
  assert.equal(readyToCum(state), true);
  playPhase(state, 'CUM');
  assert.equal(readyToCum(state), false);

  const idle = new SceneStateData();

  idle.cumThreshold = 1;
  thrusted(idle, 'thrust', 0.4);
  assert.equal(idle.progress, 0, 'no progress outside the sex phase');
}

{
  const state = new SceneStateData();
  let finished = 0;
  const hooks = { cumFinished: () => { finished++; }, soloFinished: () => { finished += 10; } };

  state.inScene = true;
  playPhase(state, 'INTRO');
  clipFinished(state, false, hooks);
  assert.equal(state.introIndex, 1);
  clipFinished(state, true, hooks);
  assert.equal(state.phase, 'HAVING_SEX');
  playPhase(state, 'LAYING_DOWN');
  clipFinished(state, false, hooks);
  assert.equal(state.phase, 'BED_IDLE');
  playPhase(state, 'CUM');
  clipFinished(state, false, hooks);
  assert.equal(finished, 1);
  state.phase = 'STATIONARY';
  state.stationaryLoopThreshold = 1;
  clipFinished(state, false, hooks);
  assert.equal(state.stationaryLoop, 1);
  clipFinished(state, false, hooks);
  assert.equal(finished, 11, 'the last loop of a solo scene ends it');
}

// --- clip names and the catalogue ---

assert.deepEqual(parseClipName('animation.doggy.fast_2'), { sceneId: 'doggy', role: 'fast', index: 1 });
assert.deepEqual(parseClipName('animation.doggy.slow_L_swap'), { sceneId: 'doggy', role: 'slow_L_swap', index: 0 }, 'the longest key wins');
assert.deepEqual(parseClipName('animation.doggy.lay_on_bed'), { sceneId: 'doggy', role: 'lay_on_bed', index: 0 });
assert.equal(parseClipName('animation.doggy.slow_l'), null, 'keys are matched exactly: slow_l is not slow_L');
assert.equal(parseClipName('animation.doggy.slo'), null);
assert.equal(parseClipName('doggy.fast'), null);
assert.equal(parseClipName('animation.doggy.fast_0'), null);

{
  const clip = (name: string) => ({ name, duration: 1, loopMode: 0, channels: [], events: [] });
  const files = [
    { path: 'data/x/asset_sync/strut/animations/scenes/doggy.strutanim', clips: [clip('animation.doggy.slow'), clip('animation.doggy.fast'), clip('animation.doggy.cum')] },
    { path: 'data/x/asset_sync/strut/animations/scenes/doggy-amy.strutanim', clips: [clip('animation.doggy.slow_2'), clip('animation.doggy.slow')] }
  ];
  const replace = new SceneCatalogue(files, 'doggy', 'amy', true);

  assert.deepEqual(replace.of('slow'), ['animation.doggy.slow', 'animation.doggy.slow_2'], 'her file has the slow clips in order of their numbers');
  assert.deepEqual(replace.of('fast'), [], 'a scene that replaces the defaults has only hers');

  const merge = new SceneCatalogue(files, 'doggy', 'amy', false);

  assert.deepEqual(merge.of('fast'), ['animation.doggy.fast'], 'a scene that does not replace falls back to the shared clips');
  assert.deepEqual(new SceneCatalogue(files, 'doggy', 'bob', true).of('cum'), ['animation.doggy.cum'], 'a girl with no file of her own plays the shared one');
}

// --- the pace prototype ---

{
  const settings = { slowBand: [0.14, 0.42] as [number, number], fastBand: [0.6, 0.9] as [number, number], slowSeconds: 16, fastSeconds: 9, blendSeconds: 3, fillSeconds: 80, minClipSeconds: 1.6, hasFast: true };
  const fresh = () => new PaceGame(settings);

  /* Holding raises the pace; letting go holds it a moment and then eases it off slowly, so a tap now and then keeps it steady. */
  const game = fresh();

  game.held = true;
  for (let i = 0; i < 12; i++) game.step(0.05, true, 20, 6);
  assert.ok(Math.abs(game.target - 0.42) < 1e-9, 'held for 0.6 s the input has taken the pace 0.7 a second');
  assert.equal(game.pace, game.target, 'and the pace is there with it: the rules have no lag');

  const held = game.target;

  game.held = false;
  for (let i = 0; i < 6; i++) game.step(0.05, true, 20, 6);
  assert.ok(game.target >= held - 0.01, 'let go, it holds for a moment');
  for (let i = 0; i < 20; i++) game.step(0.05, true, 20, 6);
  assert.ok(game.target < held - 0.15 && game.target > 0, 'and then eases off slowly');

  /* The clip does not flicker: a pace wobbling around the change is held to one clip for at least a moment. */
  const wobble = fresh();
  let changes = 0;
  let before = wobble.fast;

  wobble.pace = 0.49;

  for (let i = 0; i < 200; i++) {
    wobble.held = i % 2 === 0;
    wobble.pace = wobble.target = 0.49 + (i % 2 === 0 ? 0.04 : -0.04);
    wobble.step(0.05, true, 20, 6);

    if (wobble.fast !== before) { changes++; before = wobble.fast; }
  }

  assert.ok(changes <= 10 / 1.6 + 1, `the clip changed ${changes} times in ten seconds of wobbling`);

  /* Her band: slow, a blend, fast, a blend back. */
  const bands = fresh();

  assert.deepEqual(bands.spot(0), { lo: 0.14, hi: 0.42 });
  assert.deepEqual(bands.spot(20), { lo: 0.6, hi: 0.9 });
  assert.ok(Math.abs(bands.spot(14.5).lo - 0.37) < 1e-9, 'half way through the change it is half way between');
  assert.deepEqual(bands.spot(25 + 25), { lo: 0.14, hi: 0.42 }, 'and it repeats');

  /* Her pleasure: full speed in the band (more in the fast one), slow when too slow, falling when overwhelmed. */
  const at = (pace: number, seconds: number) => { const g = fresh(); g.pace = g.target = pace; g.held = true; g.step(0.0001, true, seconds, 80); return g; };
  const rate = (pace: number, seconds: number) => { const g = fresh(); g.pace = g.target = pace; g.held = pace > 0.5; return g.step(0.05, true, seconds, 80) / 0.05; };

  assert.ok(Math.abs(rate(0.3, 0) - 1) < 0.05, 'in the slow band the bar (80) in fillSeconds (80) is 1 a second');
  assert.ok(Math.abs(rate(0.75, 20) - 1.5) < 0.1, 'in the fast band it builds a half as fast again');
  assert.ok(rate(0.95, 20) < 0, 'overwhelmed: it falls');
  assert.ok(rate(0.2, 20) > 0 && rate(0.2, 20) < 0.3, 'too slow for what she wants: it builds slowly');
  assert.equal(fresh().step(0.05, false, 0, 80), 0, 'nothing builds outside the sex phase');
  assert.equal(at(0.75, 20).zone, 'good');

  /* The player's own excitement climbs at a high pace, cools at a low one, and the way it ends depends on it. */
  const edge = fresh();

  edge.pace = edge.target = 1;
  edge.held = true;
  for (let i = 0; i < 100; i++) edge.step(0.05, true, 20, 6);
  assert.ok(edge.edge > 0.5 && edge.edge < 1, 'five seconds at full pace builds it up');
  edge.held = false;
  edge.pace = edge.target = 0.1;
  const high = edge.edge;

  for (let i = 0; i < 100; i++) edge.step(0.05, true, 0, 6);
  assert.ok(edge.edge < high, 'a slow pace cools it');

  edge.edge = 0.7;
  assert.equal(edge.ending(1), 'together', 'finishing with her bar full and the edge in its window is together');
  edge.edge = 0.2;
  assert.equal(edge.ending(1), 'good', 'with the edge low it is just good');
  assert.equal(edge.ending(0.6), 'early', 'before her bar is full it is early');

  /* A press on the beat builds a combo, a press off the beat ends it. */
  const combo = fresh();

  assert.equal(combo.beat(0.1), true);
  assert.equal(combo.beat(0.05), true);
  assert.equal(combo.combo, 2);
  assert.equal(combo.beat(0.5), false);
  assert.equal(combo.combo, 0);
}

// --- the feel of a scene ---

{
  const clip = (name: string, duration: number, thrusts: number) => ({ name, duration, loopMode: 1, channels: [], events: Array.from({ length: thrusts }, (_, i) => ({ time: (i * duration) / thrusts, kind: 'instruction', payload: 'thrust', authority: 0 })) });
  const files = (clips: ReturnType<typeof clip>[]) => [{ path: 'data/x/asset_sync/strut/animations/scenes/doggy.strutanim', clips }];
  const lookup = (clips: ReturnType<typeof clip>[]) => (name: string) => clips.find(c => c.name === name) ?? null;

  /* A scene with a slow 4 s loop of 2 thrusts and a fast 2 s loop of 4: beats of 2 s and 0.5 s. */
  const both = [clip('animation.doggy.slow', 4, 2), clip('animation.doggy.fast', 2, 4)];
  const feel = deriveFeel(new SceneCatalogue(files(both), 'doggy', 'amy', true), lookup(both), 6, false);

  assert.equal(feel.settings.slowSeconds, 12, 'a stretch is three loops of the slow clip');
  assert.equal(feel.settings.fastSeconds, 8, 'and four of the fast');
  assert.equal(feel.settings.fillSeconds, 40, 'it fills in what the classic game would take (60 cues of half a second, a quarter more), at least 40 s');
  assert.equal(feel.settings.hasFast, true);
  assert.ok(feel.summary.includes('Slow beat 2.0 s') && feel.summary.includes('fast 0.5 s'));

  /* A slow scene with long loops takes longer, and clips are held longer before they change. */
  const slowScene = [clip('animation.doggy.slow', 9, 3), clip('animation.doggy.fast', 7, 7)];
  const slowFeel = deriveFeel(new SceneCatalogue(files(slowScene), 'doggy', 'amy', true), lookup(slowScene), 8, false);

  assert.equal(slowFeel.settings.slowSeconds, 27);
  assert.equal(slowFeel.settings.fastSeconds, 16, 'capped');
  assert.ok(slowFeel.settings.minClipSeconds > feel.settings.minClipSeconds, 'longer clips are kept longer');
  assert.ok(slowFeel.settings.fillSeconds > feel.settings.fillSeconds);

  /* A scene with no fast clip has only the slow pace, with a wide band, and the pace game does not ask for fast. */
  const onlySlow = [clip('animation.doggy.slow', 4, 2)];
  const plain = deriveFeel(new SceneCatalogue(files(onlySlow), 'doggy', 'amy', true), lookup(onlySlow), 6, false);

  assert.equal(plain.settings.hasFast, false);
  assert.equal(plain.settings.fastSeconds, 0);
  assert.deepEqual(new PaceGame(plain.settings).spot(30), { lo: 0.2, hi: 0.7 }, 'her band never moves to the fast one');

  const noFast = new PaceGame(plain.settings);

  noFast.target = noFast.pace = 1;
  noFast.held = true;
  for (let i = 0; i < 100; i++) noFast.step(0.05, true, 10, 6);
  assert.equal(noFast.fast, false, 'and she never plays a fast clip she does not have');
}

// --- the whole pace game through the engine ---

{
  const clip = (name: string, duration: number, thrusts: number) => ({ name, duration, loopMode: 1, channels: [], events: Array.from({ length: thrusts }, (_, i) => ({ time: (i * duration) / thrusts, kind: 'instruction', payload: 'thrust', authority: 0 })) });
  const sceneClips = [clip('animation.t.slow', 4, 2), clip('animation.t.fast', 2, 4), clip('animation.t.cum', 2, 0), clip('animation.t.intro', 2, 0)];
  const files = [{ path: 'data/x/asset_sync/strut/animations/scenes/t.strutanim', clips: sceneClips }];
  const info = (type: SceneInfo['type'], minigame: 'classic' | 'pace'): SceneInfo => ({ id: 't', type, needsToStrip: false, cumThreshold: 6, useKeyFrameEvents: false, replaceDefault: true, bedOffset: 0, amountOfLoops: 2, minigame, pace: { ...DEFAULT_PACE, fillSeconds: 20, minClipSeconds: 1.2 } });
  const make = (type: SceneInfo['type'], minigame: 'classic' | 'pace' = 'pace') => {
    const played: (string | null)[] = [];
    const stage: Stage = { hasClip: () => true, playClip: name => { played.push(name); }, setModel: () => undefined, wearsClothes: true, setRigVisible: () => undefined, setBed: () => undefined };
    const sim = new SceneSim(info(type, minigame), new SceneCatalogue(files, 't', 'amy', true), stage, () => undefined, () => undefined);

    return { sim, played };
  };
  const run = (sim: SceneSim, seconds: number, policy: (sim: SceneSim, t: number) => void) => { for (let i = 0; i < seconds * 20; i++) { policy(sim, i / 20); sim.tick(); } };

  /* A player who keeps her pace in her band fills her bar and may finish; the combo and the edge are tracked on the way. */
  {
    const { sim } = make('on_player');

    sim.start();
    sim.skip();
    assert.equal(sim.state.phase, 'HAVING_SEX', 'skipping goes straight to the sex phase');
    assert.equal(sim.director, 'IN_SCENE');

    let seconds = 0;

    while (!sim.canCum() && !sim.ending && seconds < 240) {
      /* Aim for the middle of her band: hold when below it, let go when above. */
      const { lo, hi } = sim.pace.spot((sim.age * 50) / 1000);

      sim.setThrusting(sim.pace.pace < (lo + hi) / 2);
      sim.tick();
      seconds += 0.05;
    }

    assert.equal(sim.canCum(), true, `her bar fills (it took ${seconds.toFixed(0)} s)`);
    assert.equal(sim.ending, null, 'a player who follows her band does not finish too soon');
    assert.ok(seconds >= 10 && seconds < 60, 'in about the time the scene asks for (fillSeconds is 20)');
  }

  /* The edge window decides the ending, the rhythm and the combo decide what it is worth. */
  {
    const { sim } = make('on_player');

    sim.start();
    sim.skip();
    sim.state.progress = sim.state.cumThreshold;
    sim.pace.edge = 0.7;
    sim.pace.best = 12;
    sim.pace.cues = 10;
    sim.pace.hits = 9;
    assert.equal(sim.canCum(), true);
    sim.cum();
    assert.equal(sim.ending, 'together');
    assert.equal(sim.reward, 2, 'together (1.5) with the best combo (+0.5)');
    assert.equal(sim.state.phase, 'CUM');
  }

  /* Holding at full pace cannot be kept up: it ends before she is ready, and is worth less. */
  {
    const { sim } = make('on_player');

    sim.start();
    sim.skip();
    run(sim, 30, game => game.setThrusting(true));
    assert.equal(sim.ending, 'early');
    assert.equal(sim.reward, 0);
  }

  /* A trigger or stick sets the pace outright and lets go back to the key. */
  {
    const { sim } = make('on_player');

    sim.start();
    sim.skip();
    sim.setAnalog(0.5);
    run(sim, 2, () => undefined);
    assert.ok(Math.abs(sim.pace.target - 0.5) < 0.02, 'the target is where the trigger is');
    sim.setAnalog(0);
    assert.equal(sim.pace.analog, null);
  }

  /* A solo scene skips to its loop and runs out its loops. */
  {
    const loopClips = [clip('animation.t.loop', 2, 0)];
    const stage: Stage = { hasClip: () => true, playClip: () => undefined, setModel: () => undefined, wearsClothes: true, setRigVisible: () => undefined, setBed: () => undefined };
    const sim = new SceneSim(info('stationary', 'pace'), new SceneCatalogue([{ path: 'data/x/asset_sync/strut/animations/scenes/t.strutanim', clips: loopClips }], 't', 'amy', true), stage, () => undefined, () => undefined);

    sim.start();
    assert.equal(sim.state.phase, 'STATIONARY');
    sim.skip();
    assert.equal(sim.state.phase, 'STATIONARY', 'a solo scene has no intro to skip once it is going');
    for (let i = 0; i < 3; i++) sim.clipEnded();
    assert.equal(sim.finished, 'complete', 'its loops run out and it ends');
  }

  /* The classic game is untouched: the thrust cues fill the bar and nothing else does. */
  {
    const { sim } = make('on_player', 'classic');

    sim.start();
    sim.skip();
    sim.setThrusting(true);
    assert.equal(sim.state.thrusting, true, 'thrusting is the input itself');
    for (let i = 0; i < 70; i++) sim.cue('thrust');
    assert.equal(sim.canCum(), true, 'seventy thrust cues of a tenth fill a bar of six');
    run(sim, 5, () => undefined);
    assert.equal(sim.pace.pace, 0, 'and the pace game plays no part');
  }
}

console.log('the simulator rules hold');
