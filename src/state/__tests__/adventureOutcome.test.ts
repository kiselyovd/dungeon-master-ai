import { beforeEach, expect, it } from 'vitest';
import { applyAdventureToolOutcome } from '../adventureOutcome';
import { useStore } from '../useStore';

const campaignId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const sessionId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const heroId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
const outcome = () => ({
  schema_version: 1,
  ruleset_id: 'srd-5.2.1',
  campaign_id: campaignId,
  session_id: sessionId,
  character_id: heroId,
  revision: 3,
  adventure: {
    state: {
      package_id: 'bell-beneath-water',
      location: 'quay',
      facts: ['ferry_route'],
      ending: null,
      closed: false,
      history: [{ kind: 'move', location: 'quay' }],
    },
    available_actions: [{ kind: 'move', location: 'town' }],
  },
  inventory: [{ id: 'rope', name: 'Rope', count: 1 }],
});
beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  useStore.getState().session.activate(campaignId, sessionId);
  useStore.getState().campaigns.setActive({
    id: campaignId,
    last_session_id: sessionId,
    revision: 2,
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    title: 'Bell',
    premise: '',
    seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
    created_at: '2026-10-06',
    updated_at: '2026-10-06',
  });
  useStore.getState().pc.replaceFromDraft({ characterId: heroId });
});
it('accepts the confirmed same-campaign outcome atomically without cancelling narration', () => {
  const controller = new AbortController();
  useStore.getState().chat.appendUser('I go to the quay');
  useStore.getState().chat.beginStream(controller);
  let notifications = 0;
  const unsubscribe = useStore.subscribe(() => notifications++);
  expect(applyAdventureToolOutcome(outcome())).toBe(true);
  unsubscribe();
  expect(notifications).toBe(1);
  expect(controller.signal.aborted).toBe(false);
  expect(useStore.getState().chat.isStreaming).toBe(true);
  expect(useStore.getState().chat.messages).toHaveLength(1);
  expect(useStore.getState().session.adventure?.state.location).toBe('quay');
  expect(useStore.getState().campaigns.active?.revision).toBe(3);
});
it('ignores malformed, cross-campaign and obsolete outcomes', () => {
  expect(applyAdventureToolOutcome({ ...outcome(), campaign_id: heroId })).toBe(false);
  expect(applyAdventureToolOutcome({ ...outcome(), revision: 1 })).toBe(false);
  expect(
    applyAdventureToolOutcome({
      ...outcome(),
      inventory: [{ id: 'gold', name: 'Gold', count: -10 }],
    }),
  ).toBe(false);
  expect(useStore.getState().campaigns.active?.revision).toBe(2);
});
