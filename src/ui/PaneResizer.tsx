import { useEffect, useRef, useState } from 'react';
import styles from './PaneResizer.module.css';

interface Props {
  label: string;
  value: number;
  min: number;
  max: number;
  onCommit: (width: number) => void;
  onPreview: (width: number) => void;
}
/** Uses the same visual handle as chat and commits only at the end of a drag. */
export function PaneResizer({ label, value, min, max, onCommit, onPreview }: Props) {
  const drag = useRef<{ id: number; x: number; value: number } | null>(null);
  const [dragging, setDragging] = useState(false);
  const preview = useRef(onPreview);
  preview.current = onPreview;
  const finish = (x?: number) => {
    const start = drag.current;
    if (!start) return;
    const width =
      x === undefined ? start.value : Math.min(max, Math.max(min, start.value + start.x - x));
    drag.current = null;
    setDragging(false);
    onPreview(width);
    if (x !== undefined) onCommit(width);
  };
  useEffect(() => {
    const cancel = () => {
      const start = drag.current;
      if (!start) return;
      drag.current = null;
      setDragging(false);
      preview.current(start.value);
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') cancel();
    };
    window.addEventListener('blur', cancel);
    document.addEventListener('visibilitychange', cancel);
    window.addEventListener('keydown', key);
    return () => {
      window.removeEventListener('blur', cancel);
      document.removeEventListener('visibilitychange', cancel);
      window.removeEventListener('keydown', key);
      if (drag.current) preview.current(drag.current.value);
    };
  }, []);
  return (
    // biome-ignore lint/a11y/useSemanticElements: focusable interactive separator follows the existing chat resize handle pattern.
    <button
      type="button"
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      className={`${styles.handle}${dragging ? ` ${styles.dragging}` : ''}`}
      style={{ marginLeft: 0 }}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.stopPropagation();
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = { id: event.pointerId, x: event.clientX, value };
        setDragging(true);
      }}
      onPointerMove={(event) => {
        if (drag.current?.id === event.pointerId)
          onPreview(
            Math.min(max, Math.max(min, drag.current.value + drag.current.x - event.clientX)),
          );
      }}
      onPointerUp={(event) => {
        if (drag.current?.id === event.pointerId) finish(event.clientX);
      }}
      onPointerCancel={() => finish()}
      onKeyDown={(event) => {
        const delta = event.shiftKey ? 64 : 16;
        const width =
          event.key === 'ArrowLeft'
            ? value + delta
            : event.key === 'ArrowRight'
              ? value - delta
              : event.key === 'Home'
                ? min
                : event.key === 'End'
                  ? max
                  : null;
        if (width !== null) {
          event.preventDefault();
          event.stopPropagation();
          onCommit(Math.min(max, Math.max(min, width)));
        }
      }}
    />
  );
}
