import { expect, it } from 'vitest';
import { DEFAULT_CHAT_WIDTH, MAX_CHAT_WIDTH, MIN_CHAT_WIDTH } from '../../../state/settings';
import { tableChatWidth, tableScopeKey } from './layout';
import type { TableScope } from './types';

it('shares the existing chat preference limits rather than defining another layout scale', () => {
  expect(tableChatWidth(Number.NaN)).toBe(DEFAULT_CHAT_WIDTH);
  expect(tableChatWidth('corrupt')).toBe(DEFAULT_CHAT_WIDTH);
  expect(tableChatWidth(1)).toBe(MIN_CHAT_WIDTH);
  expect(tableChatWidth(10_000)).toBe(MAX_CHAT_WIDTH);
});
it('keeps the UI scope stable for public revision updates and changes it for restored timelines or encounters', () => {
  const scope: TableScope = {
    mode: 'solo',
    campaignId: 'campaign',
    sessionId: 'session',
    revision: 3,
    timeline: 0,
    roomId: null,
    roomEpoch: null,
    encounterId: null,
  };
  expect(tableScopeKey(scope)).toBe(tableScopeKey({ ...scope, revision: 4 }));
  expect(tableScopeKey(scope)).not.toBe(tableScopeKey({ ...scope, timeline: 1 }));
  expect(tableScopeKey(scope)).not.toBe(tableScopeKey({ ...scope, encounterId: 'encounter' }));
});
