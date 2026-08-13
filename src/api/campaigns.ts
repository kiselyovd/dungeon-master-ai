import * as v from 'valibot';
import { backendUrl } from './client';
import {
  CAMPAIGN_RULESET_ID,
  CAMPAIGN_SCHEMA_VERSION,
  type Campaign,
  CampaignListSchema,
  CampaignSchema,
  type CampaignSummary,
  type CreateCampaignInput,
} from './contracts/campaign';
import { ChatError } from './errors';

async function request(path: string, init?: RequestInit): Promise<unknown> {
  const url = await backendUrl(path);
  let response: Response;
  try {
    response = await fetch(url, init);
  } catch (error) {
    throw ChatError.from(error);
  }
  if (!response.ok) {
    throw new ChatError('http_error', `campaign request failed (${response.status})`);
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
