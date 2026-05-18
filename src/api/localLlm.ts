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
}

export interface ManifestResponse {
  system: SystemEntry[];
  user: UserEntry[];
  installed_ids: string[];
  download_states: Record<string, DownloadStateWire>;
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
