import { useEffect, useState } from 'react';

export interface PendingUpdate {
  version: string;
  notes: string;
  install: () => Promise<void>;
}

const CHECK_DELAY_MS = 30_000;

export function useUpdater() {
  const [pending, setPending] = useState<PendingUpdate | null>(null);

  useEffect(() => {
    let cancelled = false;
    const timer = setTimeout(async () => {
      try {
        const mod = await import('@tauri-apps/plugin-updater');
        const update = await mod.check();
        if (cancelled || !update) return;
        setPending({
          version: update.version,
          notes: update.body ?? '',
          install: () => update.downloadAndInstall(),
        });
      } catch (err) {
        // Updater plugin is `active: false` until first GA release ships a
        // signed `latest.json`; the check is best-effort. Demote to debug so
        // the dev console stays clean.
        console.debug('updater check skipped', err);
      }
    }, CHECK_DELAY_MS);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, []);

  return { pending, dismiss: () => setPending(null) };
}
