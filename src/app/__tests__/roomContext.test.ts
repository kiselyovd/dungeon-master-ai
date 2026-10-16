import { expect, it } from 'vitest';
import fixture from '../../api/__fixtures__/roomUpdate.json';
import type { RoomState } from '../../api/contracts/room';
import { roomContext } from '../roomContext';

it('uses only the requested member sheet and same-campaign public context', () => {
  const state = structuredClone(fixture.state) as RoomState;
  const member = state.actors.find(
    (actor) => actor.character_id === state.own_character?.characterId,
  );
  if (!member) throw Error('fixture owner missing');
  state.journal = [
    {
      id: 'public',
      campaign_id: state.room.campaign_id,
      chapter: null,
      entry_html: '<p>Discovered</p>',
      created_at: '2026-10-13T00:00:00Z',
    },
    {
      id: 'other',
      campaign_id: 'other-campaign',
      chapter: null,
      entry_html: 'Wrong campaign',
      created_at: '2026-10-13T00:00:00Z',
    },
  ];
  const context = roomContext(state, member.member_id);
  expect(context.own?.characterId).toBe(member.character_id);
  expect(context.journal.map((entry) => entry.id)).toEqual(['public']);
  expect(() =>
    roomContext(
      state,
      state.actors.find((actor) => actor.member_id !== member.member_id)?.member_id ?? 'missing',
    ),
  ).toThrow('room_context_actor_invalid');
});
