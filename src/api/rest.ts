import * as v from 'valibot';
import { backendUrl } from './client';
import { type RestRequest, RestResultSchema, RestStatusSchema } from './contracts/rest';
import { ChatError } from './errors';

async function request(path: string, init?: RequestInit): Promise<unknown> {
  let response: Response;
  try {
    response = await fetch(await backendUrl(path), init);
  } catch (error) {
    throw ChatError.from(error);
  }
  if (!response.ok) {
    let code = 'rest_request_failed';
    try {
      const body = await response.json();
      if (typeof body?.error?.code === 'string') code = body.error.code;
    } catch {}
    throw new ChatError(response.status >= 500 ? 'provider_error' : 'http_error', code);
  }
  try {
    return await response.json();
  } catch {
    throw new ChatError('invalid_response', 'rest_response_invalid');
  }
}
export async function getRestStatus(campaignId: string, signal?: AbortSignal) {
  return v.parse(
    RestStatusSchema,
    await request(
      `/campaigns/${encodeURIComponent(campaignId)}/rest`,
      signal ? { signal } : undefined,
    ),
  );
}
export async function performRest(campaignId: string, input: RestRequest) {
  return v.parse(
    RestResultSchema,
    await request(`/campaigns/${encodeURIComponent(campaignId)}/rest`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ schema_version: 1, ruleset_id: 'srd-5.2.1', ...input }),
    }),
  );
}
