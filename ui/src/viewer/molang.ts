/*
 * A small Molang evaluator, enough for the expressions an animation's keyframes carry: numbers, + - * / %, comparisons, && || !, `a ? b : c`, `math.*` functions, `query.anim_time` and
 * `query.life_time`, and `variable.x = ...;` statements with `return`. A query or variable it does not know is 0, which is what the game gives it on a bone that has none.
 *
 * Molang's trigonometry is in <b>degrees</b>: `math.sin(90)` is 1. A value is a number; a boolean is 1 or 0.
 */

export interface MolangContext {
  /** Seconds into the clip: `query.anim_time`. */
  animTime: number;
  /** Seconds since the clip began: `query.life_time`. The viewer has no difference between the two. */
  lifeTime: number;
  /** `variable.*` and `temp.*`, kept between statements of one expression. */
  variables?: Map<string, number>;
}

type Token = { type: 'num'; value: number } | { type: 'name'; value: string } | { type: 'op'; value: string };

const DEGREES = Math.PI / 180;

function tokenize(source: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;

  while (i < source.length) {
    const c = source[i];

    if (/\s/.test(c)) { i++; continue; }

    if (/[0-9.]/.test(c)) {
      let j = i;

      while (j < source.length && /[0-9.]/.test(source[j])) j++;

      if (source[j] === 'e' || source[j] === 'E') {
        j++;

        if (source[j] === '+' || source[j] === '-') j++;

        while (j < source.length && /[0-9]/.test(source[j])) j++;
      }

      tokens.push({ type: 'num', value: Number(source.slice(i, j)) });
      i = j;
      continue;
    }

    if (/[A-Za-z_]/.test(c)) {
      let j = i;

      while (j < source.length && /[A-Za-z0-9_.]/.test(source[j])) j++;

      tokens.push({ type: 'name', value: source.slice(i, j).toLowerCase() });
      i = j;
      continue;
    }

    const two = source.slice(i, i + 2);

    if (['&&', '||', '==', '!=', '<=', '>=', '??'].includes(two)) {
      tokens.push({ type: 'op', value: two });
      i += 2;
      continue;
    }

    tokens.push({ type: 'op', value: c });
    i++;
  }

  return tokens;
}

/** A parsed expression is a function of the context, so one expression is parsed once and evaluated every frame. */
type Node = (context: MolangContext, variables: Map<string, number>) => number;

class Parser {
  private at = 0;

  constructor(private readonly tokens: Token[]) {}

  private peek(): Token | undefined { return this.tokens[this.at]; }
  private isOp(value: string): boolean { const t = this.peek(); return t?.type === 'op' && t.value === value; }
  private eat(value: string): boolean { if (this.isOp(value)) { this.at++; return true; } return false; }

  /** Statements separated by `;`. The value is that of `return`, or of the last statement. */
  program(): Node {
    const statements: Node[] = [];
    let returning = -1;

    while (this.peek()) {
      if (this.peek()?.type === 'name' && (this.peek() as { value: string }).value === 'return') {
        this.at++;
        returning = statements.length;
      }

      statements.push(this.statement());

      if (!this.eat(';')) break;
    }

    return (context, variables) => {
      let value = 0;

      for (let i = 0; i < statements.length; i++) {
        value = statements[i](context, variables);

        if (i === returning) return value;
      }

      return value;
    };
  }

  private statement(): Node {
    const token = this.peek();

    if (token?.type === 'name' && /^(variable|v|temp|t)\./.test(token.value) && this.tokens[this.at + 1]?.type === 'op' && (this.tokens[this.at + 1] as { value: string }).value === '=') {
      const name = token.value.replace(/^(v|variable)\./, 'variable.').replace(/^(t|temp)\./, 'temp.');

      this.at += 2;

      const value = this.ternary();

      return (context, variables) => { const v = value(context, variables); variables.set(name, v); return v; };
    }

    return this.ternary();
  }

  private ternary(): Node {
    const condition = this.or();

    if (!this.eat('?')) return condition;

    const yes = this.ternary();

    /* `a ? b` with no else is 0 when false. */
    const no = this.eat(':') ? this.ternary() : () => 0;

    return (context, variables) => (condition(context, variables) !== 0 ? yes(context, variables) : no(context, variables));
  }

  private or(): Node {
    let left = this.and();

    while (this.eat('||')) { const a = left; const b = this.and(); left = (c, v) => (a(c, v) !== 0 || b(c, v) !== 0 ? 1 : 0); }

    while (this.eat('??')) { const a = left; const b = this.and(); left = (c, v) => a(c, v) || b(c, v); }

    return left;
  }

  private and(): Node {
    let left = this.equality();

    while (this.eat('&&')) { const a = left; const b = this.equality(); left = (c, v) => (a(c, v) !== 0 && b(c, v) !== 0 ? 1 : 0); }

    return left;
  }

  private equality(): Node {
    let left = this.comparison();

    for (;;) {
      if (this.eat('==')) { const a = left; const b = this.comparison(); left = (c, v) => (a(c, v) === b(c, v) ? 1 : 0); }
      else if (this.eat('!=')) { const a = left; const b = this.comparison(); left = (c, v) => (a(c, v) !== b(c, v) ? 1 : 0); }
      else return left;
    }
  }

  private comparison(): Node {
    let left = this.sum();

    for (;;) {
      if (this.eat('<=')) { const a = left; const b = this.sum(); left = (c, v) => (a(c, v) <= b(c, v) ? 1 : 0); }
      else if (this.eat('>=')) { const a = left; const b = this.sum(); left = (c, v) => (a(c, v) >= b(c, v) ? 1 : 0); }
      else if (this.eat('<')) { const a = left; const b = this.sum(); left = (c, v) => (a(c, v) < b(c, v) ? 1 : 0); }
      else if (this.eat('>')) { const a = left; const b = this.sum(); left = (c, v) => (a(c, v) > b(c, v) ? 1 : 0); }
      else return left;
    }
  }

  private sum(): Node {
    let left = this.product();

    for (;;) {
      if (this.eat('+')) { const a = left; const b = this.product(); left = (c, v) => a(c, v) + b(c, v); }
      else if (this.eat('-')) { const a = left; const b = this.product(); left = (c, v) => a(c, v) - b(c, v); }
      else return left;
    }
  }

  private product(): Node {
    let left = this.unary();

    for (;;) {
      if (this.eat('*')) { const a = left; const b = this.unary(); left = (c, v) => a(c, v) * b(c, v); }
      else if (this.eat('/')) { const a = left; const b = this.unary(); left = (c, v) => { const d = b(c, v); return d === 0 ? 0 : a(c, v) / d; }; }
      else if (this.eat('%')) { const a = left; const b = this.unary(); left = (c, v) => { const d = b(c, v); return d === 0 ? 0 : a(c, v) % d; }; }
      else return left;
    }
  }

  private unary(): Node {
    if (this.eat('-')) { const inner = this.unary(); return (c, v) => -inner(c, v); }
    if (this.eat('+')) return this.unary();
    if (this.eat('!')) { const inner = this.unary(); return (c, v) => (inner(c, v) === 0 ? 1 : 0); }

    return this.primary();
  }

  private primary(): Node {
    const token = this.peek();

    if (!token) return () => 0;

    if (token.type === 'num') { this.at++; const value = token.value; return () => value; }

    if (this.eat('(')) {
      const inner = this.program();

      this.eat(')');
      return inner;
    }

    if (token.type === 'name') {
      this.at++;

      const name = token.value.replace(/^(m)\./, 'math.').replace(/^(q)\./, 'query.').replace(/^(v)\./, 'variable.').replace(/^(t)\./, 'temp.');
      const args: Node[] = [];

      if (this.eat('(')) {
        if (!this.eat(')')) {
          do { args.push(this.ternary()); } while (this.eat(','));
          this.eat(')');
        }

        return (c, v) => call(name, args.map(a => a(c, v)), c);
      }

      return (c, v) => read(name, c, v);
    }

    this.at++;
    return () => 0;
  }
}

function read(name: string, context: MolangContext, variables: Map<string, number>): number {
  switch (name) {
    case 'query.anim_time': return context.animTime;
    case 'query.life_time': return context.lifeTime;
    case 'math.pi': return Math.PI;
    case 'true': return 1;
    case 'false': return 0;
    default: return variables.get(name) ?? 0;
  }
}

function call(name: string, a: number[], _context: MolangContext): number {
  const x = a[0] ?? 0;

  switch (name) {
    case 'math.sin': return Math.sin(x * DEGREES);
    case 'math.cos': return Math.cos(x * DEGREES);
    case 'math.asin': return Math.asin(x) / DEGREES;
    case 'math.acos': return Math.acos(x) / DEGREES;
    case 'math.atan': return Math.atan(x) / DEGREES;
    case 'math.atan2': return Math.atan2(x, a[1] ?? 0) / DEGREES;
    case 'math.abs': return Math.abs(x);
    case 'math.sqrt': return Math.sqrt(Math.max(0, x));
    case 'math.exp': return Math.exp(x);
    case 'math.ln': return x > 0 ? Math.log(x) : 0;
    case 'math.floor': return Math.floor(x);
    case 'math.ceil': return Math.ceil(x);
    case 'math.round': return Math.round(x);
    case 'math.trunc': return Math.trunc(x);
    case 'math.pow': return Math.pow(x, a[1] ?? 1);
    case 'math.min': return Math.min(x, a[1] ?? x);
    case 'math.max': return Math.max(x, a[1] ?? x);
    case 'math.mod': return a[1] ? x % a[1] : 0;
    case 'math.clamp': return Math.min(Math.max(x, a[1] ?? x), a[2] ?? x);
    case 'math.lerp': return x + ((a[1] ?? x) - x) * (a[2] ?? 0);
    case 'math.sign': return Math.sign(x);
    case 'math.to_rad': return x * DEGREES;
    case 'math.to_deg': return x / DEGREES;
    /* Random has no place in a preview that must look the same every frame: the middle of the range. */
    case 'math.random': return ((a[0] ?? 0) + (a[1] ?? 1)) / 2;
    case 'math.die_roll': case 'math.die_roll_integer': return ((a[1] ?? 1) + (a[2] ?? 6)) / 2 * (a[0] ?? 1);
    default: return 0;
  }
}

const cache = new Map<string, Node>();

/** Evaluates an expression at a moment of a clip. A malformed expression is 0 rather than an error: the viewer is for looking, and the checks say what is wrong. */
export function evaluate(expression: string, context: MolangContext): number {
  let node = cache.get(expression);

  if (!node) {
    try {
      node = new Parser(tokenize(expression)).program();
    } catch {
      node = () => 0;
    }

    cache.set(expression, node);
  }

  try {
    const result = node(context, context.variables ?? new Map());

    return Number.isFinite(result) ? result : 0;
  } catch {
    return 0;
  }
}
