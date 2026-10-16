export interface DmReplySnapshot {
  assistantMessages: string[];
  successfulToolResults?: number;
  isStreaming: boolean;
  errorCode?: string;
}

export type DmReplyState =
  | { status: 'pending' }
  | { status: 'complete'; text: string }
  | { status: 'error'; code: string };

const PLACEHOLDER_MESSAGES = new Set(['...', '…']);

export function evaluateDmReply(
  snapshot: DmReplySnapshot,
  baselineAssistantCount: number,
  baselineSuccessfulToolResults = 0,
): DmReplyState {
  if (snapshot.errorCode) return { status: 'error', code: snapshot.errorCode };
  if (snapshot.isStreaming) return { status: 'pending' };

  if ((snapshot.successfulToolResults ?? 0) > baselineSuccessfulToolResults) {
    return { status: 'complete', text: '' };
  }

  const newestMessage = snapshot.assistantMessages.at(-1)?.trim() ?? '';
  if (
    snapshot.assistantMessages.length <= baselineAssistantCount ||
    newestMessage.length === 0 ||
    PLACEHOLDER_MESSAGES.has(newestMessage)
  ) {
    return { status: 'pending' };
  }

  return { status: 'complete', text: newestMessage };
}
