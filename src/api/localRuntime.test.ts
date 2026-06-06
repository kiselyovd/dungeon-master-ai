import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setBackendPortForTesting } from './client';
import {
  fetchLocalRuntimeStatus,
  persistLocalModeConfig,
  startLocalRuntimes,
  stopLocalRuntimes,
} from './localRuntime';

const PORT = 45678;
const base = `http://127.0.0.1:${PORT}`;

function mockFetch(response: Partial<Response>): ReturnType<typeof vi.fn> {
  const fn = vi.fn().mockResolvedValue({
    ok: true,
    status: 200,
    json: async () => ({}),
    ...response,
  });
  vi.stubGlobal('fetch', fn);
  return fn;
}

describe('localRuntime API client', () => {
  beforeEach(() => {
    setBackendPortForTesting(PORT);
  });
  afterEach(() => {
    setBackendPortForTesting(null);
    vi.unstubAllGlobals();
  });

  it('fetchLocalRuntimeStatus hits the backend port and returns the snapshot', async () => {
    const snap = { llm: { state: 'ready', port: 1 }, image: { state: 'off' } };
    const fn = mockFetch({ ok: true, status: 200, json: async () => snap });
    const result = await fetchLocalRuntimeStatus();
    expect(fn).toHaveBeenCalledWith(`${base}/local/runtime/status`);
    expect(result).toEqual(snap);
  });

  it('startLocalRuntimes POSTs to the backend port', async () => {
    const fn = mockFetch({ ok: true, status: 200 });
    await startLocalRuntimes();
    expect(fn).toHaveBeenCalledWith(`${base}/local/runtime/start`, { method: 'POST' });
  });

  it('stopLocalRuntimes POSTs to the backend port', async () => {
    const fn = mockFetch({ ok: true, status: 200 });
    await stopLocalRuntimes();
    expect(fn).toHaveBeenCalledWith(`${base}/local/runtime/stop`, { method: 'POST' });
  });

  it('persistLocalModeConfig POSTs the JSON body to the backend port', async () => {
    const fn = mockFetch({ ok: true, status: 200 });
    await persistLocalModeConfig({ selected_llm: 'qwen3_5_4b', vram_strategy: 'auto-swap' });
    expect(fn).toHaveBeenCalledWith(`${base}/local-mode/config`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ selected_llm: 'qwen3_5_4b', vram_strategy: 'auto-swap' }),
    });
  });

  it('throws when the backend returns a non-ok status', async () => {
    mockFetch({ ok: false, status: 500 });
    await expect(startLocalRuntimes()).rejects.toThrow('HTTP 500');
  });
});
