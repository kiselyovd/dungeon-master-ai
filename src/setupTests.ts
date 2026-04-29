import '@testing-library/jest-dom/vitest';
import { vi } from 'vitest';

/**
 * tauri-plugin-store is unavailable in jsdom because it speaks to the Tauri
 * runtime. The persist middleware in `src/state/persistStorage.ts` constructs
 * `LazyStore` instances at module load, so this mock has to be in place
 * before any test imports the store.
 *
 * Each `LazyStore` keeps an in-memory `Map`; instances created with the same
 * filename share the same bucket so the producer/consumer split that the real
 * plugin offers is preserved across reads and writes within a single test run.
 */
vi.mock('@tauri-apps/plugin-store', () => {
  const buckets = new Map<string, Map<string, unknown>>();

  class LazyStore {
    private data: Map<string, unknown>;
    constructor(filename: string) {
      let bucket = buckets.get(filename);
      if (!bucket) {
        bucket = new Map();
        buckets.set(filename, bucket);
      }
      this.data = bucket;
    }
    async get<T>(key: string): Promise<T | undefined> {
      return this.data.get(key) as T | undefined;
    }
    async set(key: string, value: unknown): Promise<void> {
      this.data.set(key, value);
    }
    async delete(key: string): Promise<boolean> {
      return this.data.delete(key);
    }
    async save(): Promise<void> {
      // no-op in tests; in production this hits the disk
    }
  }

  return { LazyStore };
});
