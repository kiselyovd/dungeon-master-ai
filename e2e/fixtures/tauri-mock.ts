import type { Page } from '@playwright/test';

/**
 * Seed the renderer with a fake `__TAURI_INTERNALS__` that satisfies the
 * `@tauri-apps/plugin-store`, `@tauri-apps/plugin-stronghold`, and
 * `@tauri-apps/api/event` clients. Each call returns the bare minimum the
 * client expects so the persist middleware can rehydrate without crashing.
 *
 * Pass `seed` to pre-populate values that the persist middleware will read
 * back on first hydration. Keys correspond to the `KEY_*` constants in
 * `src/state/persistStorage.ts` (e.g. `onboarding_completed`,
 * `active_provider`, ...).
 *
 * The store/stronghold buckets are backed by `window.localStorage` so they
 * survive a `page.reload()` - `addInitScript` re-runs on every navigation,
 * so an in-closure Map would reset and no e2e test could verify that
 * persisted state survives a restart (M11-DM Batch A, audit F3).
 */

/**
 * Pipe browser console output and uncaught errors into the Playwright test
 * runner stdout. Without this, anything `console.error`'d during persist
 * rehydration (which is where Onboarding-bypass mocks usually fail) stays
 * invisible on CI and you get a 90-second timeout with no clue why.
 */
export function pipeBrowserConsole(page: Page): void {
  page.on('console', (msg) => {
    // eslint-disable-next-line no-console
    console.log(`[browser-${msg.type()}] ${msg.text()}`);
  });
  page.on('pageerror', (err) => {
    // eslint-disable-next-line no-console
    console.log(`[browser-error] ${err.message}\n${err.stack ?? ''}`);
  });
}

export async function mockTauri(page: Page, seed: Record<string, unknown> = {}): Promise<void> {
  pipeBrowserConsole(page);

  // SplashOverlay polls /health every 250ms and only dismisses once it gets
  // a 200 OK (or after a 30-second timeout). Return a benign 200 so the
  // splash drops within the first tick and the rest of the UI becomes
  // interactive. Tests that need a real sidecar response will override
  // this with their own page.route call.
  await page.route('**/health', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{"ok":true}' });
  });

  await page.addInitScript((initialSeed: Record<string, unknown>) => {
    // Store and Stronghold state is backed by localStorage so it survives
    // a page reload (addInitScript re-runs on every navigation, so an
    // in-closure Map would reset). Store buckets are keyed by store
    // filename; Stronghold buckets by client name.
    const STORE_PREFIX = 'dmai-e2e-store:';
    const HOLD_PREFIX = 'dmai-e2e-hold:';

    const readBucket = (storageKey: string): Record<string, unknown> => {
      const raw = window.localStorage.getItem(storageKey);
      if (!raw) return {};
      try {
        return JSON.parse(raw) as Record<string, unknown>;
      } catch {
        return {};
      }
    };
    const writeBucket = (storageKey: string, bucket: Record<string, unknown>): void => {
      window.localStorage.setItem(storageKey, JSON.stringify(bucket));
    };

    // rid -> store filename. Recreated each load; the backing data lives
    // in localStorage so the rid mapping does not need to persist.
    const ridToPath = new Map<number, string>();
    let nextRid = 1;

    // Seed a freshly-loaded store bucket once, without clobbering values
    // written in a previous page life.
    const seedEntries = Object.entries(initialSeed);
    const ensureSeeded = (storageKey: string): void => {
      if (window.localStorage.getItem(storageKey) !== null) return;
      const bucket: Record<string, unknown> = {};
      for (const [k, val] of seedEntries) bucket[k] = val;
      writeBucket(storageKey, bucket);
    };

    type InvokeArgs = {
      rid?: number;
      path?: string;
      key?: string;
      value?: unknown;
      client?: string;
    };

    const internals = {
      transformCallback: () => 0,
      invoke: async (cmd: string, args: InvokeArgs = {}) => {
        // The frontend calls `invoke('backend_port')` to learn where the
        // axum sidecar is listening, then opens /health on it.
        if (cmd === 'backend_port') return 31415;

        // plugin-store: bucket keyed by the store filename (args.path).
        if (cmd === 'plugin:store|load') {
          const path = args.path ?? 'default.json';
          ensureSeeded(STORE_PREFIX + path);
          const rid = nextRid++;
          ridToPath.set(rid, path);
          return rid;
        }
        if (cmd === 'plugin:store|get_store') return null;
        if (cmd.startsWith('plugin:store|')) {
          const path = args.rid !== undefined ? ridToPath.get(args.rid) : undefined;
          if (path === undefined) {
            if (cmd === 'plugin:store|get') return [null, false];
            if (
              cmd === 'plugin:store|keys' ||
              cmd === 'plugin:store|values' ||
              cmd === 'plugin:store|entries'
            ) {
              return [];
            }
            if (cmd === 'plugin:store|length') return 0;
            return null;
          }
          const storageKey = STORE_PREFIX + path;
          const bucket = readBucket(storageKey);
          if (cmd === 'plugin:store|get') {
            const value = bucket[args.key ?? ''];
            return value === undefined ? [null, false] : [value, true];
          }
          if (cmd === 'plugin:store|set') {
            bucket[args.key ?? ''] = args.value;
            writeBucket(storageKey, bucket);
            return null;
          }
          if (cmd === 'plugin:store|has') return Object.hasOwn(bucket, args.key ?? '');
          if (cmd === 'plugin:store|delete') {
            const had = Object.hasOwn(bucket, args.key ?? '');
            delete bucket[args.key ?? ''];
            writeBucket(storageKey, bucket);
            return had;
          }
          if (cmd === 'plugin:store|save') return null;
          if (cmd === 'plugin:store|clear' || cmd === 'plugin:store|reset') {
            writeBucket(storageKey, {});
            return null;
          }
          if (cmd === 'plugin:store|keys') return Object.keys(bucket);
          if (cmd === 'plugin:store|values') return Object.values(bucket);
          if (cmd === 'plugin:store|entries') return Object.entries(bucket);
          if (cmd === 'plugin:store|length') return Object.keys(bucket).length;
          if (cmd === 'plugin:store|close') return null;
          return null;
        }

        // plugin-stronghold: byte records keyed by client name.
        if (cmd.startsWith('plugin:stronghold|')) {
          const storageKey = HOLD_PREFIX + (args.client ?? 'default');
          if (cmd === 'plugin:stronghold|get_store_record') {
            const bucket = readBucket(storageKey);
            const value = bucket[args.key ?? ''];
            return value === undefined ? null : value;
          }
          if (cmd === 'plugin:stronghold|save_store_record') {
            const bucket = readBucket(storageKey);
            bucket[args.key ?? ''] = args.value;
            writeBucket(storageKey, bucket);
            return null;
          }
          if (cmd === 'plugin:stronghold|remove_store_record') {
            const bucket = readBucket(storageKey);
            delete bucket[args.key ?? ''];
            writeBucket(storageKey, bucket);
            return null;
          }
          // initialize / load_client / create_client / save / destroy:
          // benign null so the JS client wrappers resolve.
          return null;
        }

        // event/path/window plugins called during boot return null.
        if (cmd.startsWith('plugin:event|')) return 0;
        if (cmd.startsWith('plugin:path|')) return '/mock/app-data-dir';
        if (cmd.startsWith('plugin:window|')) return null;

        return null;
      },
    };
    (window as unknown as { __TAURI_INTERNALS__: typeof internals }).__TAURI_INTERNALS__ =
      internals;

    // The Tauri event plugin lives in a sibling global. listen() resolves
    // to a function that calls __TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener
    // on cleanup; without this shim the persist hydrator throws on every
    // unmount with "Cannot read properties of undefined (reading
    // 'unregisterListener')" and blocks the UI from settling.
    (
      window as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => void } }
    ).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: () => {},
    };
  }, seed);
}
