import { beforeEach, expect, it } from 'vitest';
import { isPlayerToken } from '../../features/combat/model/selectors';
import { buildCampaignState } from '../campaignProjection';
import { applyCombatOutcome } from '../combatOutcome';
import { useStore } from '../useStore';

const campaign = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const session = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const character = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
function outcome() {
  return {
    schema_version: 1,
    encounter_id: 'enc',
    revision: 1,
    campaign_scope: {
      campaign_id: campaign,
      session_id: session,
      character_id: character,
      revision: 4,
    },
    snapshot: {
      active: true,
      round: 1,
      current_combatant: 'hero-token',
      initiative: [{ id: 'hero-token', roll: 18, dex_tiebreak: 2 }],
      combatants: [
        {
          id: 'hero-token',
          character_id: character,
          name: 'Mira',
          max_hp: 12,
          current_hp: 4,
          temp_hp: 0,
          ac: 16,
          speed_ft: 35,
          initiative_roll: 18,
          dex_mod: 2,
          conditions: [],
          budget: { action: true, bonus_action: true, reaction: true, movement_ft: 35 },
          is_dead: false,
          position: { x: 8, y: 8 },
        },
      ],
    },
    events: [],
  };
}
beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  useStore.getState().campaigns.setActive({
    id: campaign,
    title: 'Bell',
    premise: 'Ferry',
    seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
    last_session_id: session,
    revision: 3,
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    created_at: '2026-10-07T00:00:00Z',
    updated_at: '2026-10-07T00:00:00Z',
  });
  useStore.getState().session.activate(campaign, session);
  useStore.getState().pc.replaceFromDraft({
    characterId: character,
    name: 'Mira',
    heroClass: 'fighter',
    hp: 7,
    hpMax: 12,
  });
});
it('mirrors the confirmed bound HP and revision without resetting a chat stream', () => {
  const controller = new AbortController();
  useStore.getState().chat.beginStream(controller);
  const generation = useStore.getState().session.transitionGeneration;
  expect(applyCombatOutcome(outcome())).toBe(true);
  expect(useStore.getState().pc.hp).toBe(4);
  expect(useStore.getState().campaigns.active?.revision).toBe(4);
  expect(useStore.getState().chat.isStreaming).toBe(true);
  expect(controller.signal.aborted).toBe(false);
  expect(useStore.getState().session.transitionGeneration).toBe(generation);
});

it('a terminal DM combat projection closes the fight and updates its story during narration', () => {
  const controller = new AbortController();
  useStore.getState().chat.beginStream(controller);
  expect(applyCombatOutcome(outcome())).toBe(true);
  const terminal = outcome();
  terminal.revision = 2;
  terminal.campaign_scope.revision = 5;
  terminal.snapshot.active = false;
  const adventure = {
    state: {
      package_id: 'bell-beneath-water' as const,
      location: 'quay' as const,
      facts: ['ferry_route' as const],
      ending: null,
      closed: false,
      history: [],
      defense: 'retreated' as const,
      defense_encounter_id: null,
    },
    available_actions: [],
  };
  expect(applyCombatOutcome({ ...terminal, adventure })).toBe(true);
  expect(useStore.getState().combat.active).toBe(false);
  expect(useStore.getState().session.adventure?.state.defense).toBe('retreated');
  expect(useStore.getState().chat.isStreaming).toBe(true);
  expect(controller.signal.aborted).toBe(false);
});
it('rejects another character and an obsolete campaign revision', () => {
  const wrong = outcome();
  wrong.campaign_scope.character_id = 'dddddddd-dddd-4ddd-8ddd-dddddddddddd';
  expect(applyCombatOutcome(wrong)).toBe(false);
  const stale = outcome();
  stale.campaign_scope.revision = 2;
  expect(applyCombatOutcome(stale)).toBe(false);
  expect(useStore.getState().pc.hp).toBe(7);
});

it('continuation preserves the owner binding and recognizes the restored token', () => {
  const active = useStore.getState().campaigns.active;
  if (!active) throw new Error('fixture campaign missing');
  const restored = buildCampaignState({
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    campaign: active,
    session_id: session,
    character_id: character,
    projection: {
      messages: [],
      character: JSON.parse(JSON.stringify(useStore.getState().pc)),
      combat: outcome(),
      scene: null,
      journal: [],
      npcs: [],
      adventure: null,
    },
  });
  expect(restored.combat?.campaign_scope?.character_id).toBe(character);
  useStore.getState().restoration.apply(restored);
  const token = useStore.getState().combat.tokens[0];
  expect(token?.characterId).toBe(character);
  if (!token) throw new Error('restored token missing');
  expect(isPlayerToken(token, character, 'Mira')).toBe(true);
});
