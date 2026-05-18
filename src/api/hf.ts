/**
 * HTTP client for the `/hf/*` endpoints introduced in M9-DM Task 16.
 *
 * Wraps token storage (`/hf/token`), Hugging Face search (`/hf/search`),
 * gated-license checks (`/hf/model/license/{*repo_id}`), and manifest
 * mutations (`/hf/manifest/*`). Follows the same plain-`fetch` + `backendUrl`
 * pattern as `src/api/localLlm.ts` since the project has no `apiFetch`
 * helper.
 */

import { backendUrl } from './client';

export interface TokenStatus {
  connected: boolean;
  prefix?: string;
}

async function jsonRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const url = await backendUrl(path);
  const res = await fetch(url, init);
  if (!res.ok) {
    throw new Error(`${init.method ?? 'GET'} ${path} HTTP ${res.status}`);
  }
  if (res.status === 204) {
    return undefined as unknown as T;
  }
  return (await res.json()) as T;
}

export async function getTokenStatus(): Promise<TokenStatus> {
  return jsonRequest<TokenStatus>('/hf/token/status', { method: 'GET' });
}

export async function setToken(token: string): Promise<TokenStatus> {
  return jsonRequest<TokenStatus>('/hf/token', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ token }),
  });
}

export async function clearToken(): Promise<void> {
  const url = await backendUrl('/hf/token');
  const res = await fetch(url, { method: 'DELETE' });
  if (!res.ok) {
    throw new Error(`DELETE /hf/token HTTP ${res.status}`);
  }
}

export interface HfModelSibling {
  filename: string;
  size?: number;
}

export interface HfModel {
  repo_id: string;
  likes: number;
  downloads: number;
  gated: boolean;
  tags: string[];
  last_modified?: string;
  siblings: HfModelSibling[];
}

export interface SearchParams {
  q: string;
  arch?: string;
  quant?: string;
  size?: 'small' | 'medium' | 'large';
  license?: string;
  sort?: 'downloads' | 'likes' | 'last-modified';
}

export async function search(params: SearchParams): Promise<HfModel[]> {
  const query = new URLSearchParams();
  query.set('q', params.q);
  if (params.arch) query.set('arch', params.arch);
  if (params.quant) query.set('quant', params.quant);
  if (params.size) query.set('size', params.size);
  if (params.license) query.set('license', params.license);
  if (params.sort) query.set('sort', params.sort);
  return jsonRequest<HfModel[]>(`/hf/search?${query.toString()}`, { method: 'GET' });
}

export interface LicenseCheck {
  gated: boolean;
  accepted: boolean;
}

/**
 * Check whether the current HF token has accepted the gated-model license for
 * `repoId`. The backend route is `/hf/model/license/{*repo_id}` (wildcard at
 * tail, see `crates/app-server/src/lib.rs`), so the embedded `/` in
 * `org/model` is passed through verbatim rather than URL-encoded.
 */
export async function checkLicense(repoId: string): Promise<LicenseCheck> {
  return jsonRequest<LicenseCheck>(`/hf/model/license/${repoId}`, { method: 'GET' });
}

export interface AddManifestBody {
  repo_id: string;
  hf_filename: string;
  arch: string;
  quant: string;
  size_gb: number;
  license: string;
  display_name: string;
  force?: boolean;
}

export async function addManifest(body: AddManifestBody): Promise<void> {
  const url = await backendUrl('/hf/manifest/add');
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(`POST /hf/manifest/add HTTP ${res.status}`);
  }
}
