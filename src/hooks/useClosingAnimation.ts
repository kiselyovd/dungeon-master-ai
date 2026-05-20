import { useCallback, useEffect, useRef, useState } from 'react';

/**
 * Duration (ms) of the overlay enter/exit animation.
 *
 * MUST stay in sync with the CSS `--t-slow` token in src/styles/theme.css.
 * True single-source across CSS and TS is not possible without build tooling,
 * so a named constant on each side plus this comment is the goal.
 */
export const OVERLAY_CLOSE_MS = 280;

/**
 * Manages an exit animation lifecycle for overlay components.
 *
 * When `triggerClose` is called the hook sets `isClosing` to `true` so the
 * consumer can apply a closing CSS class / data-state, then fires `onClose`
 * after `durationMs` so the parent can unmount the overlay.
 *
 * Idempotent: calling `triggerClose` a second time while already closing
 * does nothing - `onClose` is called exactly once.
 *
 * On unmount any pending timer is cancelled; `onClose` is NOT called and no
 * state update occurs after unmount.
 */
export function useClosingAnimation(
  onClose: () => void,
  durationMs = OVERLAY_CLOSE_MS,
): {
  isClosing: boolean;
  triggerClose: () => void;
} {
  const [isClosing, setIsClosing] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Ref (not state) so two triggerClose() calls in the same event-loop tick
  // (before setIsClosing causes a re-render) cannot schedule two timers.
  const closingRef = useRef(false);
  const onCloseRef = useRef(onClose);

  // Keep the ref current so stale closures in setTimeout always call the
  // latest version of onClose.
  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, []);

  const triggerClose = useCallback(() => {
    if (closingRef.current) return;
    closingRef.current = true;
    setIsClosing(true);
    timerRef.current = setTimeout(() => {
      timerRef.current = null;
      closingRef.current = false;
      setIsClosing(false);
      onCloseRef.current();
    }, durationMs);
  }, [durationMs]);

  return { isClosing, triggerClose };
}
