/**
 * HTTP wiring for the multi-provider settings endpoints exposed by app-server.
 *
 * The backend speaks snake_case on the wire but our frontend types are
 * camelCase, so we translate at this boundary. Adding a new provider kind in
 * the future means one new branch here, in `assertNeverProvider`-driven
 * exhaustive switches.
 */

import { assertNeverProvider, type ProviderConfig, type ProviderKind } from '../state/providers';
import { useStore } from '../state/useStore';
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
    case 'local-mistralrs': {
      // The backend `POST /settings` for local-mistralrs needs the live LLM
      // sidecar port (see crates/app-server/src/routes/settings.rs). Read it
      // from the runtime snapshot in the Zustand store; if the runtime is
      // not ready yet the user has to start it first.
      const runtime = useStore.getState().localMode.runtime.llm;
      if (runtime.state !== 'ready') {
        throw new ChatError(
          'provider_error',
          'local runtime is not ready - start the runtime in Settings before saving.',
        );
      }
      return {
        kind: 'local-mistralrs',
        // The frontend stores the ModelId string in `modelPath`; the backend
        // accepts it as `model_id` and looks up the on-disk path itself.
        model_id: c.modelPath,
        port: runtime.port,
      };
    }
    default:
      return assertNeverProvider(c);
  }
}
