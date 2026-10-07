import * as v from 'valibot';
import { backendUrl } from './client';
import {
  type SoloRecoveryRequest,
  SoloRecoveryResultSchema,
  SoloRecoveryStatusSchema,
} from './contracts/soloRecovery';
import { ChatError } from './errors';

async function request(path: string, init?: RequestInit): Promise<unknown> {
  let response: Response;
  try {
    response = await fetch(await backendUrl(path), init);
  } catch (error) {
    throw ChatError.from(error);
  }
  if (!response.ok) {
    let code = 'recovery_request_failed';
    try {
      const body = await response.json();
      if (typeof body?.error?.code === 'string') code = body.error.code;
    } catch {}
    throw new ChatError(
      response.status === 408 || response.status === 429
        ? 'network'
        : response.status >= 500
          ? 'provider_error'
          : 'http_error',
      code,
    );
  }
  try {
    return await response.json();
  } catch {
    throw new ChatError('invalid_response', 'recovery_response_invalid');
  }
}
export async function getSoloRecoveryStatus(campaignId: string, signal?: AbortSignal) {
  return v.parse(
    SoloRecoveryStatusSchema,
    await request(
      `/campaigns/${encodeURIComponent(campaignId)}/recovery`,
      signal ? { signal } : undefined,
    ),
  );
}
export async function performSoloRecovery(campaignId: string, input: SoloRecoveryRequest) {
  return v.parse(
    SoloRecoveryResultSchema,
    await request(`/campaigns/${encodeURIComponent(campaignId)}/recovery`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ schema_version: 1, ruleset_id: 'srd-5.2.1', ...input }),
    }),
  );
}
