import type { RoomState } from '../api/contracts/room';
import { parseJournal, parseNpc, parsePcPatch } from '../state/campaignProjection';

/** Room state is already participant-scoped. Never substitute the solo sheet,
 * a proposal, another actor's document or private authored-world records. */
export function roomContext(state: RoomState, memberId: string) {
  const own = state.own_character ? parsePcPatch(state.own_character) : null;
  const actor = state.actors.find((actor) => actor.member_id === memberId);
  if (own && own.characterId !== actor?.character_id) throw Error('room_context_actor_invalid');
  const journal = state.journal
    .map(parseJournal)
    .filter((entry) => entry.campaign_id === state.room.campaign_id);
  const npcs = state.npcs.map(parseNpc).filter((npc) => npc.campaign_id === state.room.campaign_id);
  return { own, journal, npcs };
}
