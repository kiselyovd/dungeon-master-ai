import { useEffect, useRef, useState } from 'react';
import { backendUrl } from '../api/client';
import splashVideo from '../assets/splash.mp4';
import splashPoster from '../assets/splash.png';

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
      <video
        className="dm-splash-video"
        src={splashVideo}
        poster={splashPoster}
        autoPlay
        muted
        playsInline
        loop
      >
        <track kind="captions" />
      </video>
    </div>
  );
}
