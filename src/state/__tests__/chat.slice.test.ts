import { beforeEach, describe, expect, it } from 'vitest';
import { useStore } from '../useStore';

/**
 * B5: truncateTo action - 4 cases
 *
 * truncateTo(messageId) keeps every message BEFORE the target, removes
 * the target and everything after it, clears transient turn state, and
 * sets _nextSeq to (max surviving sequenceIndex) + 1 (or 0 if nothing
 * survives) so subsequent appends stay monotonic.
 */
describe('chat.truncateTo', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  it('truncates correctly in the middle of a conversation, leaving earlier messages intact', () => {
    // Build: user1 -> assistant1 -> user2 -> assistant2
    const chat = useStore.getState().chat;
    chat.appendUser('first question'); // sequenceIndex 0
    chat.appendAssistantDelta('first answer');
    chat.finalizeAssistant(); // sequenceIndex 1
    chat.appendUser('second question'); // sequenceIndex 2
    chat.appendAssistantDelta('second answer');
    chat.finalizeAssistant(); // sequenceIndex 3

    const msgs = useStore.getState().chat.messages;
    expect(msgs).toHaveLength(4);

    const user2Id = msgs[2]?.id;
    expect(user2Id).toBeDefined();

    // Truncate at the second user message - removes user2 + assistant2
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    useStore.getState().chat.truncateTo(user2Id!);

    const after = useStore.getState().chat.messages;
    expect(after).toHaveLength(2);
    expect(after[0]).toMatchObject({ role: 'user', content: 'first question' });
    expect(after[1]).toMatchObject({ role: 'assistant', content: 'first answer' });
  });

  it('truncating the first message empties history and resets _nextSeq to 0', () => {
    const chat = useStore.getState().chat;
    chat.appendUser('only message'); // sequenceIndex 0
    chat.appendAssistantDelta('reply');
    chat.finalizeAssistant(); // sequenceIndex 1

    const firstId = useStore.getState().chat.messages[0]?.id;
    expect(firstId).toBeDefined();

    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    useStore.getState().chat.truncateTo(firstId!);

    const state = useStore.getState().chat;
    expect(state.messages).toHaveLength(0);
    // _nextSeq must be 0 when no messages survive
    expect(state._nextSeq).toBe(0);
  });

  it('clears transient streaming/error/stream-event state on truncate', () => {
    const chat = useStore.getState().chat;
    chat.appendUser('hi'); // sequenceIndex 0
    chat.appendAssistantDelta('hello');
    chat.finalizeAssistant(); // sequenceIndex 1

    // Manually inject transient state that should be wiped
    useStore.setState((s) => ({
      chat: {
        ...s.chat,
        chatStreamEvents: [
          {
            id: 'evt1',
            toolName: 'roll_dice',
            sequenceIndex: 2,
            status: 'pending',
            args: {},
            result: null,
            isError: false,
            round: 1,
          },
        ],
        streamingAssistant: 'partial...',
        streamingReasoning: 'thinking...',
        lastError: { code: 'rate_limit', message: 'too many requests' },
        reasoningStreams: new Map([['turn1', 'some reasoning']]),
      },
    }));

    const msgId = useStore.getState().chat.messages[0]?.id;
    expect(msgId).toBeDefined();
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    useStore.getState().chat.truncateTo(msgId!);

    const state = useStore.getState().chat;
    expect(state.chatStreamEvents).toHaveLength(0);
    expect(state.streamingAssistant).toBeNull();
    expect(state.streamingReasoning).toBeNull();
    expect(state.lastError).toBeNull();
    expect(state.reasoningStreams.size).toBe(0);
  });

  it('sets _nextSeq to (max surviving sequenceIndex) + 1 after truncation', () => {
    const chat = useStore.getState().chat;
    chat.appendUser('msg A'); // sequenceIndex 0
    chat.appendAssistantDelta('reply A');
    chat.finalizeAssistant(); // sequenceIndex 1
    chat.appendUser('msg B'); // sequenceIndex 2
    chat.appendAssistantDelta('reply B');
    chat.finalizeAssistant(); // sequenceIndex 3

    const msgs = useStore.getState().chat.messages;
    const msgBId = msgs[2]?.id; // user B - sequenceIndex 2
    expect(msgBId).toBeDefined();

    // Truncate at msg B - survivors are msg A (seq 0) and reply A (seq 1)
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    useStore.getState().chat.truncateTo(msgBId!);

    const state = useStore.getState().chat;
    expect(state.messages).toHaveLength(2);
    // Max surviving sequenceIndex is 1, so _nextSeq must be 2
    expect(state._nextSeq).toBe(2);

    // Verify subsequent appends get monotonic sequence indices
    state.appendUser('new message');
    const newMsg = useStore.getState().chat.messages[2];
    expect(newMsg?.sequenceIndex).toBe(2);
  });
});
