import { useEffect, useState } from 'react';

function currentMotion() {
  const reduced =
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  return { reduced, active: document.visibilityState !== 'hidden' && document.hasFocus() };
}

/** Presentation preference only. It never pauses requests or canonical operations. */
export function usePresentationMotion() {
  const [motion, setMotion] = useState(currentMotion);
  useEffect(() => {
    const preference = window.matchMedia?.('(prefers-reduced-motion: reduce)');
    const refresh = () => {
      const next = currentMotion();
      setMotion((old) => (old.active === next.active && old.reduced === next.reduced ? old : next));
    };
    refresh();
    preference?.addEventListener?.('change', refresh);
    window.addEventListener('focus', refresh);
    window.addEventListener('blur', refresh);
    document.addEventListener('visibilitychange', refresh);
    return () => {
      preference?.removeEventListener?.('change', refresh);
      window.removeEventListener('focus', refresh);
      window.removeEventListener('blur', refresh);
      document.removeEventListener('visibilitychange', refresh);
    };
  }, []);
  return motion;
}
