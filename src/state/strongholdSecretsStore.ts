/**
 * Encrypted secrets store backed by tauri-plugin-stronghold. Drop-in replacement
 * for the secrets half of the split-storage adapter that previously used
 * `LazyStore` (plaintext JSON on disk via `tauri-plugin-store`).
 *
 * Surface intentionally mirrors the slice of `LazyStore` that
 * `persistStorage.ts` actually used - `get(key)`, `set(key, value)`,
 * `delete(key)`, `save()` - so callers do not need to know which backend
 * holds their secrets.
 *
 * Values are JSON-serialised before encryption (Stronghold's KV store is
 * byte-oriented). Rough shape on disk:
 *
 *   ${appDataDir}/dmai-vault.hold     - encrypted snapshot
 *   ${appLocalDataDir}/salt.txt       - argon2 salt (lazy-created by plugin)
 *
 * The vault password is a fixed constant - the encryption strength comes
 * from argon2 + the per-install salt, not from password secrecy. This is
 * the same model the official Tauri docs use; an attacker with both
 * filesystem access AND the binary can still recover the data, but
 * casual filesystem snooping is blocked.
 */
import { appDataDir } from '@tauri-apps/api/path';
import type { Client, Stronghold } from '@tauri-apps/plugin-stronghold';
import { Stronghold as StrongholdNs } from '@tauri-apps/plugin-stronghold';

const VAULT_FILENAME = 'dmai-vault.hold';
const VAULT_PASSWORD = 'dungeon-master-ai-default-vault-passphrase';
const CLIENT_NAME = 'dmai-secrets';

interface VaultHandle {
  stronghold: Stronghold;
  client: Client;
}

let handlePromise: Promise<VaultHandle> | null = null;

async function loadVault(): Promise<VaultHandle> {
  if (handlePromise) return handlePromise;
  handlePromise = (async () => {
    const vaultPath = `${await appDataDir()}/${VAULT_FILENAME}`;
    const stronghold = await StrongholdNs.load(vaultPath, VAULT_PASSWORD);
    let client: Client;
    try {
      client = await stronghold.loadClient(CLIENT_NAME);
    } catch {
      client = await stronghold.createClient(CLIENT_NAME);
    }
    return { stronghold, client };
  })().catch((err) => {
    handlePromise = null;
    throw err;
  });
  return handlePromise;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export const strongholdSecretsStore = {
  async get(key: string): Promise<unknown> {
    const { client } = await loadVault();
    const store = client.getStore();
    const bytes = await store.get(key);
    if (!bytes || bytes.length === 0) return undefined;
    const json = decoder.decode(new Uint8Array(bytes));
    try {
      return JSON.parse(json);
    } catch {
      return undefined;
    }
  },

  async set(key: string, value: unknown): Promise<void> {
    const { client } = await loadVault();
    const store = client.getStore();
    const bytes = Array.from(encoder.encode(JSON.stringify(value)));
    await store.insert(key, bytes);
  },

  async delete(key: string): Promise<void> {
    const { client } = await loadVault();
    const store = client.getStore();
    await store.remove(key);
  },

  async save(): Promise<void> {
    const { stronghold } = await loadVault();
    await stronghold.save();
  },

  /** Test-only: reset the cached vault handle so the next call reopens it. */
  _resetForTests(): void {
    handlePromise = null;
  },
};
