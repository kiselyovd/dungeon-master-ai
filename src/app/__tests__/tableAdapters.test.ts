import * as v from 'valibot';
import { expect, it } from 'vitest';
import wire from '../../api/__fixtures__/roomUpdate.json';
import { RoomUpdateSchema } from '../../api/contracts/room';
import type { RoomConnection } from '../../api/rooms';
import { tableScopeKey } from '../../features/game-table';
import type { RoomView } from '../../features/rooms/model/RoomController';
import { emptyPcData, presetToData } from '../../state/pc';
import { roomTable, type SoloTableInput, soloTable } from '../tableAdapters';

function roomFixture() {
  const state = v.parse(RoomUpdateSchema, structuredClone(wire)).state;
  const member = state.room.members.find((member) => member.role === 'player');
  if (!member) throw Error('room member fixture missing');
  const connection: RoomConnection = {
    kind: 'guest',
    room: state.room,
    member_id: member.member_id,
    address: '127.0.0.1:1',
    join_id: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
  };
  const view: RoomView = {
    state,
    connection,
    cursor: 9,
    status: 'connected',
    busy: false,
    pending: null,
    dmText: '',
    errorCode: null,
    recovery: null,
    recoveryStatus: 'ready',
    timelineReset: 0,
  };
  return { view, connection };
}
function soloFixture(): SoloTableInput {
  const { view } = roomFixture();
  const campaign = view.state?.campaign;
  if (!campaign) throw Error('campaign fixture missing');
  return {
    campaign: {
      id: campaign.id,
      title: campaign.title,
      premise: campaign.premise,
      seed: campaign.seed,
      revision: campaign.revision,
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      last_session_id: campaign.last_session_id,
      created_at: '2026-10-12T00:00:00Z',
      updated_at: '2026-10-12T00:00:00Z',
    },
    timeline: 0,
    character: {
      ...emptyPcData(),
      ...presetToData('fighter'),
      characterId: 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',
    },
    title: null,
    map: null,
    art: null,
    tokens: [],
    activeFight: false,
    encounterId: null,
    messages: [],
    outcomes: [],
    configured: true,
    runtimeReady: true,
    busy: false,
    recovery: 'clear',
  };
}
it('requires confirmed configuration even if a local process is running', () => {
  const model = soloTable({ ...soloFixture(), configured: false });
  expect(model.availability.send).toEqual({ enabled: false, reason: 'model_required' });
  expect(model.readiness).toBe('needs_model');
  expect(soloTable({ ...soloFixture(), runtimeReady: null }).readiness).toBe('unknown');
});
it('does not invent tactical geometry during exploration or an own actor from a legacy class alone', () => {
  const input = soloFixture();
  expect(soloTable(input).scene.kind).toBe('exploration');
  const missing = soloTable({ ...input, character: { ...input.character, characterId: null } });
  expect(missing.ownActorId).toBeNull();
  expect(missing.availability.send).toEqual({ enabled: false, reason: 'hero_required' });
  expect(soloTable({ ...input, encounterId: 'ended-encounter' }).combat).toBe('ended');
});
it('withholds private sheets, provider secrets, system text and arbitrary tool payloads', () => {
  const input = soloFixture();
  input.character.backstory = 'PRIVATE_BACKSTORY';
  input.messages = [
    { id: 'system', role: 'system', content: 'PRIVATE_CANON' },
    { id: 'public', role: 'assistant', content: 'Public reply' },
  ];
  input.outcomes = [
    {
      id: 'roll',
      toolName: 'roll_dice',
      args: { token: 'PRIVATE_TOKEN' },
      result: { total: 18, secret: 'PRIVATE_RESULT' },
      isError: false,
      round: 1,
      timestamp: '',
      handledBy: 'engine',
    },
  ];
  const model = soloTable(input);
  expect(JSON.stringify(model)).not.toMatch(/PRIVATE_/);
  expect(model.outcomes[0]?.values).toEqual({ total: 18 });
});
it('binds guest ownership by stable member/actor IDs and separates host administration', () => {
  const { view, connection } = roomFixture();
  const model = roomTable(view, connection);
  expect(model.ownActorId).toBe(
    view.state?.actors.find((actor) => actor.member_id === connection.member_id)?.character_id ??
      null,
  );
  expect(model.readiness).toBe('unknown'); // transport connectivity is not an inference test
  expect(model.availability.configure).toEqual({ enabled: false, reason: 'host_only' });
  if (!view.state) throw Error('room state fixture missing');
  expect(
    roomTable({ ...view, state: { ...view.state, own_character: null } }, connection).availability
      .send,
  ).toEqual({ enabled: false, reason: 'hero_required' });
  expect(JSON.stringify(model)).not.toMatch(
    /invitation|own_character|own_proposal|pending_proposals|address/,
  );
  const host: RoomConnection = {
    kind: 'host',
    room: connection.room,
    member_id:
      connection.room.members.find((member) => member.role === 'host')?.member_id ??
      connection.member_id,
    address: '127.0.0.1:1',
    invitation: 'PRIVATE_INVITATION',
  };
  expect(roomTable(view, host).scope.mode).toBe('host');
  expect(roomTable(view, host).availability.configure).toEqual({ enabled: true });
});
it('does not collapse equal display names and invalidates selection for restored room timelines/epochs', () => {
  const { view, connection } = roomFixture();
  if (!view.state) throw Error('room state fixture missing');
  view.state.actors = view.state.actors.map((actor) => ({ ...actor, name: 'Same name' }));
  const model = roomTable(view, connection);
  expect(new Set(model.actors.map((actor) => actor.id)).size).toBe(model.actors.length);
  expect(tableScopeKey(model.scope)).not.toBe(tableScopeKey({ ...model.scope, timeline: 4 }));
  expect(tableScopeKey(model.scope)).not.toBe(tableScopeKey({ ...model.scope, roomEpoch: 5 }));
  expect(roomTable({ ...view, status: 'disconnected' }, connection).availability.send).toEqual({
    enabled: false,
    reason: 'offline',
  });
});
