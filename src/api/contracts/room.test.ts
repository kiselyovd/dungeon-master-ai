import * as v from 'valibot';
import { expect, it } from 'vitest';
import wireFixture from '../__fixtures__/roomUpdate.json';
import { RoomUpdateSchema } from './room';

function fixture() {
  return structuredClone(wireFixture);
}
it('accepts a scoped current state ahead of the delivered cursor', () => {
  expect(v.parse(RoomUpdateSchema, fixture()).state.title).toBe('DM lane');
});
it('rejects cursor corruption and inconsistent campaign/session scope', () => {
  const invalid = fixture();
  invalid.cursor = invalid.head + 1;
  expect(v.safeParse(RoomUpdateSchema, invalid).success).toBe(false);
  const wrong = fixture();
  wrong.state.room.campaign_id = wrong.state.room.room_id;
  expect(v.safeParse(RoomUpdateSchema, wrong).success).toBe(false);
  const order = fixture();
  order.events = [
    { sequence: 2, kind: 'messages', payload: {} },
    { sequence: 1, kind: 'messages', payload: {} },
  ];
  expect(v.safeParse(RoomUpdateSchema, order).success).toBe(false);
});
it('rejects credential-bearing public room data', () => {
  const input = fixture();
  Object.assign(input.state.room, { capability: 'forbidden' });
  expect(v.safeParse(RoomUpdateSchema, input).success).toBe(false);
});
