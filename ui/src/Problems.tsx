import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { Chevron, SeverityIcon } from './icons';
import type { Diagnostic, Severity } from './types';

/** Every row is this tall, which is what lets a pack with thousands of problems draw only the rows in view. */
const ROW = 26;
const OVERSCAN = 8;

const ORDER: Record<Severity, number> = { error: 0, warning: 1, info: 2 };

interface Props {
  /** The height in pixels when it is open (the person can drag it). */
  height?: number;
  problems: Diagnostic[];
  checking: boolean;
  collapsed: boolean;
  onToggle: () => void;
  onJump: (problem: Diagnostic) => void;
  onRecheck: () => void;
}

export function Problems({ problems, checking, collapsed, height, onToggle, onJump, onRecheck }: Props) {
  const [hidden, setHidden] = useState<Set<Severity>>(new Set());
  const counts = useMemo(() => {
    const total: Record<Severity, number> = { error: 0, warning: 0, info: 0 };

    for (const p of problems) total[p.severity]++;

    return total;
  }, [problems]);
  const count = (severity: Severity) => counts[severity];
  const sorted = useMemo(() => [...problems].sort((a, b) => ORDER[a.severity] - ORDER[b.severity] || (a.file < b.file ? -1 : a.file > b.file ? 1 : 0) || a.line - b.line), [problems]);
  const shown = useMemo(() => sorted.filter(p => !hidden.has(p.severity)), [sorted, hidden]);
  const list = useRef<HTMLDivElement>(null);
  const [scroll, setScroll] = useState(0);
  const [viewport, setViewport] = useState(400);

  useEffect(() => {
    const element = list.current;

    if (!element) return;

    const measure = () => setViewport(element.clientHeight);
    const observer = new ResizeObserver(measure);

    measure();
    observer.observe(element);

    return () => observer.disconnect();
  }, [collapsed]);

  const first = Math.max(0, Math.floor(scroll / ROW) - OVERSCAN);
  const last = Math.min(shown.length, Math.ceil((scroll + viewport) / ROW) + OVERSCAN);

  const filter = (severity: Severity) => (
    <span
      class="count"
      style={{ opacity: hidden.has(severity) ? 0.4 : 1, cursor: 'pointer' }}
      title={`Show or hide ${severity}s`}
      onClick={event => {
        event.stopPropagation();
        setHidden(previous => { const next = new Set(previous); next.has(severity) ? next.delete(severity) : next.add(severity); return next; });
      }}
    >
      <SeverityIcon severity={severity} /> {count(severity)}
    </span>
  );

  return (
    <div class={'problems' + (collapsed ? ' collapsed' : '')} style={collapsed || !height ? undefined : { height }}>
      <div class="problems-head" onClick={onToggle}>
        <Chevron open={!collapsed} />
        <span class="title">Problems</span>
        {filter('error')}
        {filter('warning')}
        {filter('info')}
        <span class="grow" />
        {checking && <span class="muted">Checking…</span>}
        <button class="btn ghost small" onClick={event => { event.stopPropagation(); onRecheck(); }}>Check the whole pack</button>
      </div>

      {!collapsed && (
        <div class="problems-list" ref={list} onScroll={event => setScroll((event.currentTarget as HTMLDivElement).scrollTop)}>
          {problems.length === 0 && <div class="ok-line">No problems found.</div>}
          <div style={{ height: shown.length * ROW, position: 'relative' }}>
            {shown.slice(first, last).map((problem, offset) => (
              <div key={first + offset} class="problem" style={{ top: (first + offset) * ROW }} title={problem.message} onClick={() => onJump(problem)}>
                <SeverityIcon severity={problem.severity} />
                <span class="msg">{problem.message}<span class="code">{problem.code}</span></span>
                <span class="where">{problem.file}:{problem.line}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
