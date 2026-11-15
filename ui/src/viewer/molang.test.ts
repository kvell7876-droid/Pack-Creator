/*
 * The Molang evaluator, held to the expressions animations carry. Run with `npm run test:molang`: there is no test framework, only assertions.
 */
import assert from 'node:assert/strict';
import { evaluate } from './molang';

const at = (expression: string, time = 0) => evaluate(expression, { animTime: time, lifeTime: time });
const near = (actual: number, expected: number) => assert.ok(Math.abs(actual - expected) < 1e-9, `${actual} is not ${expected}`);

/* Arithmetic, precedence, unary minus (the converter wraps a negated expression as -(expr)). */
near(at('1 + 2 * 3'), 7);
near(at('(1 + 2) * 3'), 9);
near(at('-(2 + 3)'), -5);
near(at('10 / 4'), 2.5);
near(at('7 % 4'), 3);
near(at('10 / 0'), 0);

/* Molang's trigonometry is in degrees. */
near(at('math.sin(90)'), 1);
near(at('Math.cos(180)'), -1);
near(at('math.sin(query.anim_time * 90)', 1), 1);
near(at('q.anim_time * 2', 1.5), 3);

/* Comparison, logic and the conditional. */
near(at('query.anim_time > 1 ? 5 : 2', 2), 5);
near(at('query.anim_time > 1 ? 5 : 2', 0.5), 2);
near(at('1 < 2 && 2 < 3'), 1);
near(at('1 > 2 || 0'), 0);
near(at('!0'), 1);

/* Functions. */
near(at('math.clamp(5, 0, 3)'), 3);
near(at('math.lerp(0, 10, 0.25)'), 2.5);
near(at('math.min(4, 9) + math.max(1, 2)'), 6);
near(at('math.abs(-3)'), 3);
near(at('math.pow(2, 3)'), 8);

/* Statements and variables, with `return`. */
near(at('variable.x = 3; variable.x * 2'), 6);
near(at('v.a = 2; return v.a + 1;'), 3);

/* What it does not know is 0, and a broken expression is 0 and not an error. */
near(at('query.something_else'), 0);
near(at('math.nonsense(3)'), 0);
near(at('((('), 0);
near(at(''), 0);

console.log('molang: all passed');
