import { useRef } from 'preact/hooks';

/**
 * A bar between two panels that is dragged to resize them. `onStart` says to remember the current size; `onDrag` is told how far the pointer has moved since then (in
 * pixels, across or down), and the owner sets the size from that. Double click puts the size back.
 */
export function Splitter({ direction, onStart, onDrag, onEnd, onReset }: { direction: 'vertical' | 'horizontal'; onStart: () => void; onDrag: (dx: number, dy: number) => void; onEnd: () => void; onReset: () => void }) {
  const from = useRef<{ x: number; y: number } | null>(null);

  return (
    <div
      class={'splitter ' + direction}
      role="separator"
      aria-orientation={direction}
      title="Drag to resize, double click to reset"
      onPointerDown={event => {
        (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
        from.current = { x: event.clientX, y: event.clientY };
        onStart();
      }}
      onPointerMove={event => {
        if (from.current) onDrag(event.clientX - from.current.x, event.clientY - from.current.y);
      }}
      onPointerUp={() => {
        if (from.current) onEnd();
        from.current = null;
      }}
      onDblClick={onReset}
    />
  );
}
