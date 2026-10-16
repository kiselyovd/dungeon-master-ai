import { afterEach, describe, expect, it, vi } from 'vitest';
import { createCampaign, fetchCampaign, fetchCampaigns } from '../campaigns';
import type { Campaign } from '../contracts/campaign';

vi.mock('../client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://test.local${path}`),
}));

afterEach(() => {
  vi.unstubAllGlobals();
});

const campaign: Campaign = {
  id: '62ba8579-904b-4460-a68a-df6268ed404a',
  schema_version: 1,
  ruleset_id: 'srd-5.2.1',
  title: 'Серебряная долина',
  premise: 'Древняя дорога проснулась',
  seed: { kind: 'collaborative', answers: { genre: ['classic_fantasy'] } },
  last_session_id: 'b5271021-b82c-40c4-aa9f-554af0d79b22',
  revision: 1,
  created_at: '2026-08-13T03:21:00.000Z',
  updated_at: '2026-08-13T03:21:00.000Z',
};

describe('campaign API', () => {
  it('validates list, detail and create responses', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(JSON.stringify([{ ...campaign, seed: undefined, created_at: undefined }]), {
          status: 200,
        }),
      )
      .mockResolvedValueOnce(new Response(JSON.stringify(campaign), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(campaign), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);

    const listed = await fetchCampaigns();
    const fetched = await fetchCampaign(campaign.id);
    const created = await createCampaign({
      title: campaign.title,
      premise: campaign.premise,
      seed: campaign.seed,
    });

    expect(listed[0]?.last_session_id).toBe(campaign.last_session_id);
    expect(fetched.seed).toEqual(campaign.seed);
    expect(created.ruleset_id).toBe('srd-5.2.1');
    expect(JSON.parse(fetchMock.mock.calls[2]?.[1]?.body as string)).toMatchObject({
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
    });
  });

  it.each([
    ['schema version', { ...campaign, schema_version: 2 }],
    ['ruleset', { ...campaign, ruleset_id: 'legacy' }],
    ['uuid', { ...campaign, id: 'not-a-uuid' }],
    ['last session', { ...campaign, last_session_id: undefined }],
  ])('rejects invalid %s before callers can project it', async (_label, body) => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify(body), { status: 200 })),
    );

    await expect(fetchCampaign(campaign.id)).rejects.toMatchObject({
      code: 'invalid_response',
    });
  });
});
