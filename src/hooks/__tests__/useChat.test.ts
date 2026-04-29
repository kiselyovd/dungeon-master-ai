import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { type StreamChatOptions, streamChat } from '../../api/chat';
import { ChatError } from '../../api/errors';
import { useStore } from '../../state/useStore';
import { useChat } from '../useChat';

vi.mock('../../api/chat', () => ({
  streamChat: vi.fn(),
}));

const streamChatMock = vi.mocked(streamChat);

/**
 * renderHook + React 19 + testing-library v16 don't populate `result.current`
 * synchronously - the wrapper sets it inside a useEffect. This helper does the
 * tiny post-mount flush so `result.current` is guaranteed live before the
 * test reads it.
 */
async function flushMount() {
  await act(async () => {
    await Promise.resolve();
  });
}

describe('useChat', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    streamChatMock.mockReset();
  });

  it('send is a no-op for empty or whitespace input', async () => {
    const { result } = renderHook(() => useChat());
    await flushMount();

    await act(async () => {
      await result.current.send('');
    });
    await act(async () => {
      await result.current.send('   ');
    });

    expect(streamChatMock).not.toHaveBeenCalled();
    expect(useStore.getState().chat.messages).toEqual([]);
    expect(useStore.getState().chat.isStreaming).toBe(false);
  });

  it('send is a no-op while another stream is in flight', async () => {
    // Simulate the "already streaming" guard precondition by toggling the
    // chat slice directly. Avoids leaving a never-resolving streamChat
    // promise dangling between tests, which previously polluted the next
    // test's renderHook lifecycle.
    useStore.setState((s) => ({
      chat: { ...s.chat, isStreaming: true, abortController: new AbortController() },
    }));

    const { result } = renderHook(() => useChat());
    await flushMount();

    await act(async () => {
      await result.current.send('blocked');
    });

    expect(streamChatMock).not.toHaveBeenCalled();
    expect(useStore.getState().chat.messages).toEqual([]);
  });

  it('send streams text deltas into the streaming buffer and finalises on success', async () => {
    streamChatMock.mockImplementation(async (opts: StreamChatOptions) => {
      opts.onTextDelta('hello ');
      opts.onTextDelta('world');
      return { reason: 'stop' };
    });

    const { result } = renderHook(() => useChat());
    await flushMount();

    await act(async () => {
      await result.current.send('hi');
    });

    const state = useStore.getState().chat;
    // Final transcript: user message + assistant message, streaming buffer
    // cleared, isStreaming false.
    expect(state.messages).toHaveLength(2);
    expect(state.messages[0]).toMatchObject({ role: 'user', content: 'hi' });
    expect(state.messages[1]).toMatchObject({ role: 'assistant', content: 'hello world' });
    expect(state.streamingAssistant).toBeNull();
    expect(state.isStreaming).toBe(false);
    expect(state.lastError).toBeNull();
  });

  it('send forwards a ChatError into the chat slice without losing the user message', async () => {
    streamChatMock.mockRejectedValue(new ChatError('rate_limit', 'too many requests'));

    const { result } = renderHook(() => useChat());
    await flushMount();

    await act(async () => {
      await result.current.send('hi');
    });

    const state = useStore.getState().chat;
    expect(state.lastError).toEqual({ code: 'rate_limit', message: 'too many requests' });
    // User message stays so the user can see what was sent.
    expect(state.messages).toHaveLength(1);
    expect(state.messages[0]).toMatchObject({ role: 'user', content: 'hi' });
    expect(state.isStreaming).toBe(false);
    expect(state.streamingAssistant).toBeNull();
  });

  it('coerces non-ChatError throwables (e.g. AbortError) into a ChatError payload', async () => {
    const abortErr = new DOMException('aborted', 'AbortError');
    streamChatMock.mockRejectedValue(abortErr);

    const { result } = renderHook(() => useChat());
    await flushMount();

    await act(async () => {
      await result.current.send('hi');
    });

    expect(useStore.getState().chat.lastError?.code).toBe('aborted');
  });

  it('cancel triggers abort on the active controller', async () => {
    let captured: AbortSignal | undefined;
    streamChatMock.mockImplementation(
      (opts: StreamChatOptions) =>
        new Promise<{ reason: string }>((_, reject) => {
          captured = opts.signal;
          opts.signal?.addEventListener('abort', () => {
            reject(new DOMException('aborted', 'AbortError'));
          });
        }),
    );

    const { result } = renderHook(() => useChat());
    await flushMount();

    // Run the send + cancel in a single act block so the streamChat promise
    // is fully resolved (via the abort path) before the test returns and
    // pollutes the next one.
    await act(async () => {
      const sendPromise = result.current.send('hi');
      // Yield once so beginStream + streamChat invocation land.
      await Promise.resolve();
      expect(captured?.aborted).toBe(false);
      result.current.cancel();
      await sendPromise;
    });

    expect(captured?.aborted).toBe(true);
    expect(useStore.getState().chat.lastError?.code).toBe('aborted');
    expect(useStore.getState().chat.isStreaming).toBe(false);
  });
});
