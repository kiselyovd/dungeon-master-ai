import { type UIEvent, useCallback, useRef, useState } from 'react';

/**
 * Auto-scroll to bottom only when the user is within `thresholdPx` of the
 * bottom. If the user scrolls up past the threshold, auto-scroll pauses;
 * it resumes the moment the user scrolls back into the threshold zone.
 */
export function useStickyScroll(thresholdPx = 100) {
  const [shouldScroll, setShouldScroll] = useState(true);
  const ref = useRef<HTMLDivElement | null>(null);

  const onScroll = useCallback(
    (e: UIEvent<HTMLElement>) => {
      const el = e.currentTarget;
      const distanceFromBottom = el.scrollHeight - el.clientHeight - el.scrollTop;
      setShouldScroll(distanceFromBottom <= thresholdPx);
    },
    [thresholdPx],
  );

  const scrollToBottom = useCallback(() => {
    const el = ref.current;
    if (!shouldScroll || !el) return;
    el.scrollTop = el.scrollHeight;
  }, [shouldScroll]);

  return { ref, onScroll, scrollToBottom, shouldScroll };
}
