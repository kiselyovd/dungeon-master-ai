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

/**
 * tauri-plugin-stronghold is also runtime-only; the underlying iota_stronghold
 * vault file cannot be opened in jsdom. The mock keeps the same per-client KV
 * map model so production callers (e.g. `strongholdSecretsStore`) get a
 * working `Stronghold.load -> loadClient -> getStore` chain.
 */
vi.mock('@tauri-apps/plugin-stronghold', () => {
  const vaults = new Map<string, Map<string, Map<string, number[]>>>();

  function getStoreFor(vaultPath: string, clientName: string): Map<string, number[]> {
    let vault = vaults.get(vaultPath);
    if (!vault) {
      vault = new Map();
      vaults.set(vaultPath, vault);
    }
    let store = vault.get(clientName);
    if (!store) {
      store = new Map();
      vault.set(clientName, store);
    }
    return store;
  }

  class Store {
    constructor(private readonly bucket: Map<string, number[]>) {}
    async get(key: string): Promise<number[] | null> {
      return this.bucket.get(key) ?? null;
    }
    async insert(key: string, value: number[]): Promise<void> {
      this.bucket.set(key, value);
    }
    async remove(key: string): Promise<void> {
      this.bucket.delete(key);
    }
  }

  class Client {
    constructor(
      private readonly vaultPath: string,
      private readonly name: string,
    ) {}
    getStore(): Store {
      return new Store(getStoreFor(this.vaultPath, this.name));
    }
  }

  class Stronghold {
    private constructor(private readonly vaultPath: string) {}
    static async load(vaultPath: string, _password: string): Promise<Stronghold> {
      return new Stronghold(vaultPath);
    }
    async loadClient(name: string): Promise<Client> {
      const vault = vaults.get(this.vaultPath);
      if (!vault?.has(name)) throw new Error(`client "${name}" not found`);
      return new Client(this.vaultPath, name);
    }
    async createClient(name: string): Promise<Client> {
      // Touch the bucket to mark the client as existing for subsequent loads.
      getStoreFor(this.vaultPath, name);
      return new Client(this.vaultPath, name);
    }
    async save(): Promise<void> {
      // no-op in tests
    }
  }

  return { Stronghold, Client };
});

vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: vi.fn(async () => '/mock/app-data-dir'),
  appLocalDataDir: vi.fn(async () => '/mock/app-local-data-dir'),
}));
