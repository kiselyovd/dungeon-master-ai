/**
 * HTTP wiring for the multi-provider settings endpoints exposed by app-server.
 *
 * The backend speaks snake_case on the wire but our frontend types are
 * camelCase, so we translate at this boundary. Adding a new provider kind in
 * the future means one new branch here, in `assertNeverProvider`-driven
 * exhaustive switches.
 */

import { assertNeverProvider, type ProviderConfig, type ProviderKind } from '../state/providers';
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

interface BackendError {
  error?: { code?: string; message?: string };
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

export async function postSettings(config: ProviderConfig): Promise<ActiveProviderInfo> {
  const url = await backendUrl('/settings');
  const body = JSON.stringify(toWireConfig(config));
  let resp: Response;
  try {
    resp = await fetch(url, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body,
    });
  } catch (e) {
    throw ChatError.from(e);
  }
  if (!resp.ok) {
    const parsed = (await resp.json().catch(() => ({}) as BackendError)) as BackendError;
    const message = parsed.error?.message ?? `POST /settings HTTP ${resp.status}`;
    throw new ChatError('provider_error', message);
  }
  return (await resp.json()) as ActiveProviderInfo;
}

function toWireConfig(c: ProviderConfig): Record<string, unknown> {
  switch (c.kind) {
    case 'anthropic':
      return { kind: 'anthropic', api_key: c.apiKey, model: c.model };
    case 'openai-compat':
      return {
        kind: 'openai-compat',
        base_url: c.baseUrl,
        api_key: c.apiKey,
        model: c.model,
      };
    case 'local-mistralrs':
      throw new ChatError('provider_error', 'local-mistralrs provider is not implemented in M1.5');
    default:
      return assertNeverProvider(c);
  }
}
