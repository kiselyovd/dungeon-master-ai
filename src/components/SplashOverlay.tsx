import { useEffect, useRef, useState } from 'react';
import { backendUrl } from '../api/client';
import { KEY_ART } from '../assets/livingTabletop';

const POLL_INTERVAL_MS = 250;
const FADE_OUT_MS = 320;
const MAX_WAIT_MS = 30_000;

async function pingHealth(signal: AbortSignal): Promise<boolean> {
  try {
    const url = await backendUrl('/health');
    const res = await fetch(url, { signal });
    return res.ok;
  } catch {
    return false;
  }
}

export function SplashOverlay() {
  const [ready, setReady] = useState(false);
  const [hidden, setHidden] = useState(false);
  const fadeTimer = useRef<number | null>(null);

  useEffect(() => {
    const ac = new AbortController();
    let cancelled = false;
    const startedAt = Date.now();

    const tick = async (): Promise<void> => {
      if (cancelled) return;
      const ok = await pingHealth(ac.signal);
      if (cancelled) return;
      if (ok) {
        setReady(true);
        return;
      }
      if (Date.now() - startedAt > MAX_WAIT_MS) {
        setReady(true);
        return;
      }
      window.setTimeout(() => void tick(), POLL_INTERVAL_MS);
    };

    void tick();

    return () => {
      cancelled = true;
      ac.abort();
    };
  }, []);

  useEffect(() => {
    if (!ready) return;
    fadeTimer.current = window.setTimeout(() => setHidden(true), FADE_OUT_MS);
    return () => {
      if (fadeTimer.current !== null) {
        window.clearTimeout(fadeTimer.current);
        fadeTimer.current = null;
      }
    };
  }, [ready]);

  if (hidden) return null;

  return (
    <div
      className={`dm-splash${ready ? ' is-fading' : ''}`}
      role="status"
      aria-live="polite"
      aria-label="Loading"
    >
      <div className="dm-splash-art" data-art-direction="living-tabletop">
        <img src={KEY_ART.splash} alt="" className="dm-splash-image" />
        <span className="dm-ambient-light" aria-hidden="true" />
        <span className="dm-ambient-dust" aria-hidden="true" />
      </div>
    </div>
  );
}
