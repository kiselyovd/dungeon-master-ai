import * as v from 'valibot';
import { backendUrl } from './client';
import type { AdventureAction } from './contracts/adventure';
import {
  CAMPAIGN_RULESET_ID,
  CAMPAIGN_SCHEMA_VERSION,
  type Campaign,
  CampaignListSchema,
  CampaignSchema,
  type CampaignSummary,
  type CreateCampaignInput,
} from './contracts/campaign';
import {
  type CampaignStateResponse,
  CampaignStateSchema,
  type CommitCharacterRequest,
} from './contracts/campaignState';
import { ChatError } from './errors';

export interface AdventureActionRequest {
  sessionId: string;
  characterId: string;
  expectedRevision: number;
  causationId: string;
  action: AdventureAction;
}

export async function actInAdventure(
  campaignId: string,
  input: AdventureActionRequest,
): Promise<CampaignStateResponse> {
  return parse(
    CampaignStateSchema,
    await request(`/campaigns/${encodeURIComponent(campaignId)}/adventure/actions`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        schema_version: 1,
        ruleset_id: CAMPAIGN_RULESET_ID,
        session_id: input.sessionId,
        character_id: input.characterId,
        expected_revision: input.expectedRevision,
        causation_id: input.causationId,
        action: input.action,
      }),
    }),
  );
}

async function request(path: string, init?: RequestInit): Promise<unknown> {
  const url = await backendUrl(path);
  let response: Response;
  try {
    response = await fetch(url, init);
  } catch (error) {
    throw ChatError.from(error);
  }
  if (!response.ok) {
    throw new ChatError(
      response.status >= 500 ? 'provider_error' : 'http_error',
      `campaign request failed (${response.status})`,
    );
  }
  return response.json().catch(() => {
    throw new ChatError('invalid_response', 'campaign response is not JSON');
  });
}

function parse<TSchema extends v.BaseSchema<unknown, unknown, v.BaseIssue<unknown>>>(
  schema: TSchema,
  value: unknown,
): v.InferOutput<TSchema> {
  const result = v.safeParse(schema, value);
  if (!result.success) {
    throw new ChatError('invalid_response', 'campaign response shape is invalid');
  }
  return result.output;
}

export async function fetchCampaigns(): Promise<CampaignSummary[]> {
  return parse(CampaignListSchema, await request('/campaigns'));
}

export async function fetchCampaign(campaignId: string): Promise<Campaign> {
  return parse(CampaignSchema, await request(`/campaigns/${encodeURIComponent(campaignId)}`));
}

export async function fetchCampaignState(
  campaignId: string,
  signal?: AbortSignal,
): Promise<CampaignStateResponse> {
  return parse(
    CampaignStateSchema,
    await request(
      `/campaigns/${encodeURIComponent(campaignId)}/state`,
      signal ? { signal } : undefined,
    ),
  );
}

export async function commitCampaignCharacter(
  campaignId: string,
  input: CommitCharacterRequest,
  signal?: AbortSignal,
): Promise<CampaignStateResponse> {
  return parse(
    CampaignStateSchema,
    await request(`/campaigns/${encodeURIComponent(campaignId)}/character`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      ...(signal ? { signal } : {}),
      body: JSON.stringify({
        schema_version: 1,
        ruleset_id: CAMPAIGN_RULESET_ID,
        session_id: input.sessionId,
        character_id: input.characterId,
        expected_revision: input.expectedRevision,
        causation_id: input.causationId,
        character: input.character,
      }),
    }),
  );
}

export async function createCampaign(input: CreateCampaignInput): Promise<Campaign> {
  const response = await request('/campaigns', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      schema_version: CAMPAIGN_SCHEMA_VERSION,
      ruleset_id: CAMPAIGN_RULESET_ID,
      title: input.title,
      premise: input.premise,
      seed: input.seed,
    }),
  });
  return parse(CampaignSchema, response);
}
