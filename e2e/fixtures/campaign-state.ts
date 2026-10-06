import { randomUUID } from 'node:crypto';
import type { Page } from '@playwright/test';
import type { CampaignSeed } from '../../src/api/contracts/campaign';
import type { CampaignStateResponse } from '../../src/api/contracts/campaignState';
import { emptyPcData, type HeroClassId, PRESETS } from '../../src/state/pc';

export async function mockCampaignState(page: Page, seed: Record<string, unknown>): Promise<void> {
  const states = new Map<string, CampaignStateResponse>();
  const receipts = new Map<string, { signature: string; state: CampaignStateResponse }>();
  const makeState = (
    id: string,
    session: string,
    title: string,
    premise: string,
    source: CampaignSeed,
  ): CampaignStateResponse => ({
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    campaign: {
      id,
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title,
      premise,
      seed: source,
      last_session_id: session,
      revision: 2,
      created_at: '2026-08-13T03:21:00.000Z',
      updated_at: '2026-08-13T03:21:00.000Z',
    },
    session_id: session,
    character_id: null as string | null,
    projection: {
      character: {} as Record<string, unknown>,
      messages: [],
      combat: null,
      scene: null,
      journal: [],
      npcs: [],
    },
  });
  if (typeof seed.active_campaign_id === 'string' && typeof seed.active_session_id === 'string') {
    const state = makeState(
      seed.active_campaign_id,
      seed.active_session_id,
      'Fixture Chronicle',
      'A deterministic adventure.',
      { kind: 'collaborative', answers: { genre: ['classic_fantasy'] } },
    );
    const seededPc =
      typeof seed.pc === 'object' && seed.pc !== null ? (seed.pc as Record<string, unknown>) : {};
    const heroClass = typeof seededPc.heroClass === 'string' ? seededPc.heroClass : seed.hero_class;
    if (typeof heroClass === 'string') {
      state.character_id = '92414394-f4b1-477f-9278-10f75c3c4fb7';
      state.projection.character = {
        ...emptyPcData(),
        ...(PRESETS[heroClass as HeroClassId] ?? PRESETS.fighter),
        heroClass,
        ...seededPc,
        characterId: state.character_id,
      };
    }
    states.set(state.campaign.id, state);
  }

  await page.route(
    /^https?:\/\/[^/]+\/campaigns(?:\/[^/?]+(?:\/(?:state|character))?)?(?:\?|$)/,
    async (route) => {
      const request = route.request();
      const path = new URL(request.url()).pathname;
      const parts = path.split('/');
      const id = parts[2];
      const reply = (body: unknown, status = 200) =>
        route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });
      if (path === '/campaigns' && request.method() === 'POST') {
        const input = request.postDataJSON();
        const state = makeState(randomUUID(), randomUUID(), input.title, input.premise, input.seed);
        state.campaign.revision = 1;
        states.set(state.campaign.id, state);
        await reply(state.campaign, 201);
        return;
      }
      if (path === '/campaigns' && request.method() === 'GET') {
        await reply([...states.values()].map((state) => state.campaign));
        return;
      }
      const state = id ? states.get(id) : undefined;
      if (!state) {
        await reply({ error: { code: 'not_found' } }, 404);
        return;
      }
      if (parts[3] === 'character' && request.method() === 'PUT') {
        const input = request.postDataJSON();
        const key = `${id}:${input.causation_id}`;
        const signature = JSON.stringify(input);
        const receipt = receipts.get(key);
        if (receipt) {
          await reply(
            receipt.signature === signature ? receipt.state : { error: { code: 'conflict' } },
            receipt.signature === signature ? 200 : 409,
          );
          return;
        }
        if (
          input.session_id !== state.session_id ||
          input.expected_revision !== state.campaign.revision
        ) {
          await reply({ error: { code: 'revision_conflict' } }, 409);
          return;
        }
        state.character_id = input.character_id;
        state.projection.character = { ...input.character, characterId: input.character_id };
        state.campaign.revision += 1;
        receipts.set(key, { signature, state: structuredClone(state) });
        await reply(state);
        return;
      }
      await reply(parts[3] === 'state' ? state : state.campaign);
    },
  );
}
