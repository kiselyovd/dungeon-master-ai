import type { Page } from '@playwright/test';

/**
 * Seed the renderer with a fake `__TAURI_INTERNALS__` that satisfies the
 * `@tauri-apps/plugin-store`, `@tauri-apps/plugin-stronghold`, and
 * `@tauri-apps/api/event` clients. Each call returns the bare minimum the
 * client expects so the persist middleware can rehydrate without crashing.
 *
 * Pass `seed` to pre-populate values that the persist middleware will read
 * back on first hydration. Keys correspond to the
 * `KEY_*` constants in `src/state/persistStorage.ts` (e.g.
 * `onboarding_completed`, `active_provider`, ...).
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
  await page.addInitScript((initialSeed: Record<string, unknown>) => {
    type StoreBucket = Map<string, unknown>;
    const buckets = new Map<number, StoreBucket>();
    let nextRid = 1;
    const seedEntries = Object.entries(initialSeed);

    const allocBucket = (): number => {
      const rid = nextRid++;
      const bucket = new Map<string, unknown>(seedEntries);
      buckets.set(rid, bucket);
      return rid;
    };

    const internals = {
      transformCallback: () => 0,
      invoke: async (cmd: string, args: { rid?: number; key?: string; value?: unknown }) => {
        // plugin-store load returns a resource id for an empty store; we
        // pre-fill the bucket with the seed so the very first .get() call
        // already sees what the test asked for.
        if (cmd === 'plugin:store|load') return allocBucket();
        if (cmd === 'plugin:store|get_store') return null;
        if (cmd === 'plugin:store|get') {
          const bucket = args.rid !== undefined ? buckets.get(args.rid) : undefined;
          const value = bucket?.get(args.key ?? '');
          return value === undefined ? [null, false] : [value, true];
        }
        if (cmd === 'plugin:store|set') {
          const bucket = args.rid !== undefined ? buckets.get(args.rid) : undefined;
          bucket?.set(args.key ?? '', args.value);
          return null;
        }
        if (cmd === 'plugin:store|has') {
          const bucket = args.rid !== undefined ? buckets.get(args.rid) : undefined;
          return bucket?.has(args.key ?? '') ?? false;
        }
        if (cmd === 'plugin:store|delete') {
          const bucket = args.rid !== undefined ? buckets.get(args.rid) : undefined;
          return bucket?.delete(args.key ?? '') ?? false;
        }
        if (cmd === 'plugin:store|save') return null;
        if (cmd === 'plugin:store|clear') return null;
        if (cmd === 'plugin:store|reset') return null;
        if (cmd === 'plugin:store|keys') {
          const bucket = args.rid !== undefined ? buckets.get(args.rid) : undefined;
          return bucket ? Array.from(bucket.keys()) : [];
        }
        if (cmd === 'plugin:store|values') {
          const bucket = args.rid !== undefined ? buckets.get(args.rid) : undefined;
          return bucket ? Array.from(bucket.values()) : [];
        }
        if (cmd === 'plugin:store|entries') {
          const bucket = args.rid !== undefined ? buckets.get(args.rid) : undefined;
          return bucket ? Array.from(bucket.entries()) : [];
        }
        if (cmd === 'plugin:store|length') {
          const bucket = args.rid !== undefined ? buckets.get(args.rid) : undefined;
          return bucket?.size ?? 0;
        }
        if (cmd === 'plugin:store|close') return null;

        // plugin-stronghold: secrets are not exercised by these specs, so
        // every call returns a benign value. createClient and loadClient
        // return null (the client wrapper does not care), execute_procedure
        // and the store helpers return null/empty.
        if (cmd.startsWith('plugin:stronghold|')) {
          if (cmd === 'plugin:stronghold|get_store_record') return null;
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
