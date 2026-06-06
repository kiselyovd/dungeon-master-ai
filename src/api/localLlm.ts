/**
 * HTTP client for the `/local-llm/*` endpoints introduced in M9-DM Task 14.
 *
 * Bridges the Settings -> ModelSelector container to the backend manifest +
 * active-model setter. The wire shape mirrors `app_domain::local_llm::manifest`
 * (snake_case) + `crates/app-server/src/routes/local_llm.rs` (download_states).
 */

import type { SystemEntry, UserEntry } from '../state/local_llm/manifest';
import { backendUrl } from './client';

export interface DownloadStateWire {
  state: string;
  progress?: number;
  errorMessage?: string;
  /** True for a 401/403 HuggingFace failure; UI offers an "Add token" action. */
  authRequired?: boolean;
}

export interface ManifestResponse {
  system: SystemEntry[];
  user: UserEntry[];
  installed_ids: string[];
  download_states: Record<string, DownloadStateWire>;
}

/** Wire shape for download events streamed over GET /local-llm/download-events. */
export interface DownloadEventWire {
  /** Dotted wire id matching manifest keys, e.g. "qwen3.5-4b". */
  id: string;
  /** One of: "progress" | "completed" | "failed". */
  kind: string;
  bytes_done?: number;
  total_bytes?: number;
  reason?: string;
  /** True for a 401/403 HuggingFace failure (only on kind === "failed"). */
  auth_required?: boolean;
}

export async function fetchLocalLlmManifest(): Promise<ManifestResponse> {
  const url = await backendUrl('/local-llm/manifest');
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`GET /local-llm/manifest HTTP ${res.status}`);
  }
  return (await res.json()) as ManifestResponse;
}

export async function setActiveLocalModel(id: string): Promise<void> {
  const url = await backendUrl('/local-llm/active-model');
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ id }),
  });
  if (!res.ok) {
    throw new Error(`POST /local-llm/active-model HTTP ${res.status}`);
  }
}

/** POST /local-llm/download/:model_id - initiates a background download (202). */
export async function startModelDownload(id: string): Promise<void> {
  const url = await backendUrl(`/local-llm/download/${encodeURIComponent(id)}`);
  const res = await fetch(url, { method: 'POST' });
  if (!res.ok) {
    throw new Error(`POST /local-llm/download/${id} HTTP ${res.status}`);
  }
}

/** DELETE /local-llm/model/:model_id - cancels in-progress or removes installed (204). */
export async function cancelOrDeleteModel(id: string): Promise<void> {
  const url = await backendUrl(`/local-llm/model/${encodeURIComponent(id)}`);
  const res = await fetch(url, { method: 'DELETE' });
  if (!res.ok) {
    throw new Error(`DELETE /local-llm/model/${id} HTTP ${res.status}`);
  }
}

/**
 * GET /local-llm/download-events - subscribes to the SSE download event stream.
 *
 * Returns a cancel function that closes the underlying EventSource. The
 * callback receives each parsed `DownloadEventWire` as it arrives.
 */
export async function subscribeDownloadEvents(
  callback: (ev: DownloadEventWire) => void,
): Promise<() => void> {
  const url = await backendUrl('/local-llm/download-events');
  const es = new EventSource(url);
  es.addEventListener('download', (raw) => {
    try {
      const ev = JSON.parse((raw as MessageEvent<string>).data) as DownloadEventWire;
      callback(ev);
    } catch (err) {
      console.warn('[subscribeDownloadEvents] parse error', err);
    }
  });
  es.onerror = () => {
    // Reconnect is handled automatically by EventSource; nothing to do here.
    console.warn('[subscribeDownloadEvents] SSE error - browser will retry');
  };
  return () => {
    es.close();
  };
}
