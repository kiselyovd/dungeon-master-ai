/**
 * HTTP wiring for the providers info endpoint exposed by app-server.
 */

import type { ProviderKind } from '../state/providers';
import { backendUrl } from './client';
import { ChatError } from './errors';

export interface ActiveProviderInfo {
  kind: string;
  default_model: string;
}

export interface ProvidersInfo {
  available: ProviderKind[];
  active: ActiveProviderInfo;
}

export async function getProviders(): Promise<ProvidersInfo> {
  const url = await backendUrl('/providers');
  let resp: Response;
  try {
    resp = await fetch(url);
  } catch (e) {
    throw ChatError.from(e);
  }
  if (!resp.ok) {
    throw new ChatError('http_error', `GET /providers HTTP ${resp.status}`);
  }
  return (await resp.json()) as ProvidersInfo;
}
