/**
 * HTTP client for the Local Mode runtime + config endpoints.
 *
 * Every local-feature fetch MUST go through this module so it resolves against
 * the dynamically-discovered backend port via `backendUrl()`. Calling fetch
 * with a bare `/local*` path resolves against the Tauri webview origin and 404s -
 * see audit findings L1 / G1 ("404 on anything local"). The guard test
 * `no-bare-local-fetch.test.ts` enforces this rule.
 *
 * Mirrors `src/api/localLlm.ts` (the `/local-llm/*` client).
 */

import type { ModelId, RuntimeState, VramStrategy } from '../state/localMode';
import { backendUrl } from './client';

/** Wire shape of `GET /local/runtime/status` (backend `RegistrySnapshot`). */
export interface LocalRuntimeSnapshot {
  llm: RuntimeState;
  image: RuntimeState;
}

/** Wire shape of `POST /local-mode/config` (backend `LocalModeConfig`). */
export interface LocalModeConfigWire {
  selected_llm: ModelId;
  vram_strategy: VramStrategy;
}

/**
 * Build an Error for a non-ok response, pulling the backend's JSON error
 * message (`{error:{message}}`) into the text so a failed runtime start is
 * debuggable in the UI instead of a bare "HTTP 500". Falls back to the status
 * code when the body is absent or not the expected shape. (Audit blocker 3.)
 */
async function responseError(res: Response, method: string, path: string): Promise<Error> {
  let detail = '';
  try {
    const body = (await res.json()) as { error?: { message?: unknown }; message?: unknown };
    const msg = body?.error?.message ?? body?.message;
    if (typeof msg === 'string' && msg.trim()) detail = `: ${msg}`;
  } catch {
    // Non-JSON or empty body - keep the bare status.
  }
  return new Error(`${method} ${path} HTTP ${res.status}${detail}`);
}

/** GET /local/runtime/status - snapshot of both sidecar runtimes. */
export async function fetchLocalRuntimeStatus(): Promise<LocalRuntimeSnapshot> {
  const url = await backendUrl('/local/runtime/status');
  const res = await fetch(url);
  if (!res.ok) {
    throw await responseError(res, 'GET', '/local/runtime/status');
  }
  return (await res.json()) as LocalRuntimeSnapshot;
}

/** POST /local/runtime/start - spawn the LLM (+ optional image) sidecars. */
export async function startLocalRuntimes(): Promise<void> {
  const url = await backendUrl('/local/runtime/start');
  const res = await fetch(url, { method: 'POST' });
  if (!res.ok) {
    throw await responseError(res, 'POST', '/local/runtime/start');
  }
}

/** POST /local/runtime/stop - stop both sidecar runtimes. */
export async function stopLocalRuntimes(): Promise<void> {
  const url = await backendUrl('/local/runtime/stop');
  const res = await fetch(url, { method: 'POST' });
  if (!res.ok) {
    throw await responseError(res, 'POST', '/local/runtime/stop');
  }
}

/** POST /local-mode/config - persist the selected Qwen variant + VRAM strategy. */
export async function persistLocalModeConfig(config: LocalModeConfigWire): Promise<void> {
  const url = await backendUrl('/local-mode/config');
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(config),
  });
  if (!res.ok) {
    throw await responseError(res, 'POST', '/local-mode/config');
  }
}
