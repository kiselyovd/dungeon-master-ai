import { LazyStore } from '@tauri-apps/plugin-store';
import { beforeEach, expect, it, vi } from 'vitest';
import {
  acknowledgeCampaignCharacter,
  acknowledgeCampaignCreation,
  prepareCampaignCharacter,
  prepareCampaignCreation,
} from './campaignPreparation';

const id = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const session = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const store = new LazyStore('campaign-preparation.json');
beforeEach(async () => {
  for (const key of ['creation', `character:${id}`]) await store.delete(key);
});
it('retains the creation UUID after unknown ACK and only clears the matching attempt', async () => {
  const input = {
    title: ' Road ',
    premise: ' Rain ',
    seed: { kind: 'collaborative' as const, answers: {} },
  };
  const first = await prepareCampaignCreation(input);
  expect(await prepareCampaignCreation(input)).toEqual(first);
  await acknowledgeCampaignCreation(crypto.randomUUID());
  expect(await prepareCampaignCreation(input)).toEqual(first);
  await acknowledgeCampaignCreation(first.requestId);
  expect((await prepareCampaignCreation(input)).requestId).not.toBe(first.requestId);
});
it('fails closed before dispatch if disk persistence fails', async () => {
  const save = vi.spyOn(LazyStore.prototype, 'save').mockRejectedValueOnce(new Error('disk'));
  await expect(
    prepareCampaignCreation({
      title: 'Road',
      premise: '',
      seed: { kind: 'collaborative', answers: {} },
    }),
  ).rejects.toThrow('disk');
  save.mockRestore();
});
it('keeps character identity and original revision across a cold retry without storing portrait bytes', async () => {
  const input = {
    sessionId: session,
    characterId: null,
    expectedRevision: 1,
    character: { name: 'Mira', portraitUrl: 'data:image/png;base64,test-private' },
  };
  const first = await prepareCampaignCharacter(id, input);
  const retry = await prepareCampaignCharacter(id, { ...input, expectedRevision: 2 });
  expect(retry).toEqual(first);
  expect(JSON.stringify(await store.get(`character:${id}`))).not.toContain('test-private');
  await acknowledgeCampaignCharacter(id, 'unrelated');
  expect(await prepareCampaignCharacter(id, input)).toEqual(first);
  await acknowledgeCampaignCharacter(id, first.causationId);
  expect((await prepareCampaignCharacter(id, input)).causationId).not.toBe(first.causationId);
});
it('retains invalid persisted identity rather than silently replacing it', async () => {
  await store.set('creation', { requestId: 'broken' });
  await expect(
    prepareCampaignCreation({
      title: 'Road',
      premise: '',
      seed: { kind: 'collaborative', answers: {} },
    }),
  ).rejects.toThrow('campaign_preparation_invalid');
  expect(await store.get('creation')).toEqual({ requestId: 'broken' });
});
