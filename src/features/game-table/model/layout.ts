import { clampChatWidth, DEFAULT_CHAT_WIDTH } from '../../../state/settings';
import type { TableScope } from './types';

/** Encounters and restored timelines invalidate selection independently of
 * actor display names or a transport connection's temporary port. */
export function tableScopeKey(scope: TableScope): string {
  return JSON.stringify([
    scope.mode,
    scope.campaignId,
    scope.sessionId,
    scope.roomId,
    scope.roomEpoch,
    scope.timeline,
    scope.encounterId,
  ]);
}

export function tableChatWidth(value: unknown): number {
  return typeof value === 'number' ? clampChatWidth(value) : DEFAULT_CHAT_WIDTH;
}
