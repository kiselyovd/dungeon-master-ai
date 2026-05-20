import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { type AgentTurnOptions, streamAgentTurn } from '../../api/agent';
import { fetchSessionMessages } from '../../api/chat';
import { ChatError } from '../../api/errors';
import { useStore } from '../../state/useStore';
import { ChatPanel } from '../ChatPanel';
import toolCardStyles from '../ToolCallCard.module.css';
import '../../i18n';

vi.mock('../../api/agent', () => ({
  streamAgentTurn: vi.fn(),
}));

vi.mock('../../api/chat', () => ({
  fetchSessionMessages: vi.fn(async () => []),
}));

const streamAgentTurnMock = vi.mocked(streamAgentTurn);
const fetchSessionMessagesMock = vi.mocked(fetchSessionMessages);

describe('ChatPanel', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    streamAgentTurnMock.mockReset();
    fetchSessionMessagesMock.mockReset();
    fetchSessionMessagesMock.mockResolvedValue([]);
  });

  it('renders empty state with placeholder when no messages', () => {
    render(<ChatPanel />);
    const input = screen.getByPlaceholderText(/What do you do/i);
    expect(input).toBeInTheDocument();
  });

  it('renders existing messages from store', () => {
    useStore.setState((s) => ({
      chat: {
        ...s.chat,
        messages: [
          { id: 'm1', role: 'user', content: 'hello' },
          { id: 'm2', role: 'assistant', content: 'hi there' },
        ],
      },
    }));

    render(<ChatPanel />);
    expect(screen.getByText('hello')).toBeInTheDocument();
    expect(screen.getByText('hi there')).toBeInTheDocument();
  });

  it('shows streaming assistant text when present', () => {
    useStore.setState((s) => ({
      chat: { ...s.chat, streamingAssistant: 'in progress...' },
    }));
    render(<ChatPanel />);
    expect(screen.getByText('in progress...')).toBeInTheDocument();
  });

  it('disables send button when input is empty', () => {
    render(<ChatPanel />);
    const button = screen.getByRole('button', { name: /Send/i });
    expect(button).toBeDisabled();
  });

  it('enables send button when input has text', () => {
    render(<ChatPanel />);
    const input = screen.getByPlaceholderText(/What do you do/i);
    fireEvent.change(input, { target: { value: 'test' } });
    const button = screen.getByRole('button', { name: /Send/i });
    expect(button).toBeEnabled();
  });

  it('keeps send button disabled when only images are staged and text input is empty', async () => {
    const user = userEvent.setup();
    render(<ChatPanel />);

    // Paste an image file into the textarea to stage it.
    const input = screen.getByPlaceholderText(/What do you do/i);
    const pngBytes = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]);
    const imageFile = new File([pngBytes], 'test.png', { type: 'image/png' });

    await user.click(input);
    // Simulate clipboard paste with an image file - no text.
    fireEvent.paste(input, {
      clipboardData: {
        items: [{ kind: 'file', getAsFile: () => imageFile }],
        types: ['Files'],
        getData: () => '',
      },
    });

    // Text input is still empty, so Send must remain disabled.
    const button = screen.getByRole('button', { name: /Send/i });
    expect(button).toBeDisabled();
  });

  it('runs the full send flow: type, click Send, deltas stream, assistant message lands', async () => {
    const user = userEvent.setup();
    streamAgentTurnMock.mockImplementation(async (opts: AgentTurnOptions) => {
      opts.onTextDelta('Hello, ');
      opts.onTextDelta('traveler.');
      opts.onAgentDone(1);
    });

    render(<ChatPanel />);
    const input = screen.getByPlaceholderText(/What do you do/i);
    await user.type(input, 'I draw my sword');

    fireEvent.click(screen.getByRole('button', { name: /Send/i }));

    await waitFor(() => {
      expect(streamAgentTurnMock).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(screen.getByText('Hello, traveler.')).toBeInTheDocument();
    });

    const state = useStore.getState().chat;
    expect(state.messages).toHaveLength(2);
    expect(state.messages[0]).toMatchObject({ role: 'user', content: 'I draw my sword' });
    expect(state.messages[1]).toMatchObject({
      role: 'assistant',
      content: 'Hello, traveler.',
    });
    expect(state.isStreaming).toBe(false);

    // Draft input cleared after send.
    expect((input as HTMLTextAreaElement).value).toBe('');
  });

  it('Enter sends the draft, Shift+Enter inserts a newline', async () => {
    const user = userEvent.setup();
    streamAgentTurnMock.mockResolvedValue(undefined);

    render(<ChatPanel />);
    const input = screen.getByPlaceholderText(/What do you do/i) as HTMLTextAreaElement;

    await user.click(input);
    await user.keyboard('first line');
    await user.keyboard('{Shift>}{Enter}{/Shift}'); // Shift+Enter -> newline, no submit
    await user.keyboard('second line');

    expect(streamAgentTurnMock).not.toHaveBeenCalled();
    expect(input.value).toBe('first line\nsecond line');

    await user.keyboard('{Enter}'); // plain Enter submits

    await waitFor(() => {
      expect(streamAgentTurnMock).toHaveBeenCalledTimes(1);
    });
    expect(streamAgentTurnMock.mock.calls[0]?.[0].playerMessage).toBe('first line\nsecond line');
  });

  it('Stop button cancels an in-flight stream and surfaces an aborted error', async () => {
    const user = userEvent.setup();
    streamAgentTurnMock.mockImplementation(
      (opts: AgentTurnOptions) =>
        new Promise<void>((_, reject) => {
          opts.signal?.addEventListener('abort', () => {
            reject(new DOMException('aborted', 'AbortError'));
          });
        }),
    );

    render(<ChatPanel />);
    const input = screen.getByPlaceholderText(/What do you do/i);
    await user.type(input, 'cast fireball');

    fireEvent.click(screen.getByRole('button', { name: /Send/i }));

    // Stop button replaces Send while streaming.
    const stopButton = await screen.findByRole('button', { name: /Stop/i });
    expect(useStore.getState().chat.isStreaming).toBe(true);

    fireEvent.click(stopButton);

    await waitFor(() => {
      expect(useStore.getState().chat.isStreaming).toBe(false);
      expect(useStore.getState().chat.lastError?.code).toBe('aborted');
    });
  });

  it('renders the dm-chat-error retry bar when session.loadError is set', async () => {
    // Make the on-mount session fetch reject so useSession populates loadError.
    fetchSessionMessagesMock.mockRejectedValueOnce(new Error('backend down'));

    render(<ChatPanel />);
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveClass('dm-chat-error');
    expect(alert).toHaveTextContent(/failed to load chat history/i);
    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
  });

  it('renders an error alert when the stream rejects', async () => {
    const user = userEvent.setup();
    streamAgentTurnMock.mockRejectedValue(new ChatError('rate_limit', 'too many requests'));

    render(<ChatPanel />);
    const input = screen.getByPlaceholderText(/What do you do/i);
    await user.type(input, 'hi');
    fireEvent.click(screen.getByRole('button', { name: /Send/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(useStore.getState().chat.lastError?.code).toBe('rate_limit');
  });

  // B6: stagingError auto-dismiss
  describe('B6 stagingError auto-dismiss', () => {
    afterEach(() => {
      vi.useRealTimers();
    });

    it('stagingError clears automatically after 4 seconds', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      render(<ChatPanel />);

      const input = screen.getByPlaceholderText(/What do you do/i);
      // Paste an oversized file to trigger a staging error
      const bigFile = new File([new Uint8Array(6 * 1024 * 1024)], 'big.png', {
        type: 'image/png',
      });
      await act(async () => {
        fireEvent.paste(input, {
          clipboardData: {
            items: [{ kind: 'file', getAsFile: () => bigFile }],
            types: ['Files'],
            getData: () => '',
          },
        });
      });

      // The error message must appear
      expect(screen.getByRole('status')).toBeInTheDocument();

      // Advance 4 seconds - the error must auto-dismiss
      await act(async () => {
        vi.advanceTimersByTime(4000);
      });

      expect(screen.queryByRole('status')).not.toBeInTheDocument();
    });
  });

  // B2: inline ToolCallCard tests
  describe('inline ToolCallCard rendering', () => {
    it('(a) tool-call card renders inline in the chat history', () => {
      useStore.setState((s) => ({
        chat: {
          ...s.chat,
          chatStreamEvents: [
            {
              id: 'call_abc',
              toolName: 'roll_dice',
              sequenceIndex: 0,
              status: 'pending',
              args: {},
              result: null,
              isError: false,
              round: 1,
            },
          ],
        },
      }));

      render(<ChatPanel />);
      const card = screen.getByTestId('tool-call-card-call_abc');
      expect(card).toBeInTheDocument();
      expect(screen.getByTestId('chat-history')).toContainElement(card);
    });

    it('(b) settled card has a data-status attribute and renders the tool result', () => {
      useStore.setState((s) => ({
        chat: {
          ...s.chat,
          chatStreamEvents: [
            {
              id: 'call_settled',
              toolName: 'apply_damage',
              sequenceIndex: 0,
              status: 'success',
              args: { target: 'goblin', amount: 5 },
              result: { hp: 42 },
              isError: false,
              round: 1,
            },
          ],
        },
      }));

      render(<ChatPanel />);
      const card = screen.getByTestId('tool-call-card-call_settled');
      expect(card).toHaveAttribute('data-status', 'success');
      // The card must render the actual result value in the DOM.
      expect(card).toHaveTextContent('"hp": 42');
    });

    it('(c) tool card appears between assistant text bubbles ordered by sequenceIndex', () => {
      useStore.setState((s) => ({
        chat: {
          ...s.chat,
          messages: [
            { id: 'msg1', role: 'assistant', content: 'Before the roll', sequenceIndex: 0 },
            { id: 'msg2', role: 'assistant', content: 'After the roll', sequenceIndex: 2 },
          ],
          chatStreamEvents: [
            {
              id: 'call_mid',
              toolName: 'roll_dice',
              sequenceIndex: 1,
              status: 'success',
              args: {},
              result: { total: 17 },
              isError: false,
              round: 1,
            },
          ],
        },
      }));

      render(<ChatPanel />);
      const history = screen.getByTestId('chat-history');
      const beforeText = screen.getByText('Before the roll');
      const card = screen.getByTestId('tool-call-card-call_mid');
      const afterText = screen.getByText('After the roll');

      // Verify ordering: before < card < after in the DOM
      expect(history.compareDocumentPosition(beforeText)).toBeTruthy();
      const beforePos = beforeText.compareDocumentPosition(card);
      const afterPos = card.compareDocumentPosition(afterText);
      // Node.DOCUMENT_POSITION_FOLLOWING = 4: card is after beforeText
      expect(beforePos & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      // afterText is after card
      expect(afterPos & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    it('(d) error variant has data-status="error" and the error CSS class', () => {
      useStore.setState((s) => ({
        chat: {
          ...s.chat,
          chatStreamEvents: [
            {
              id: 'call_err',
              toolName: 'apply_damage',
              sequenceIndex: 0,
              status: 'error',
              args: {},
              result: { error: 'target not found' },
              isError: true,
              round: 1,
            },
          ],
        },
      }));

      render(<ChatPanel />);
      const card = screen.getByTestId('tool-call-card-call_err');
      expect(card).toHaveAttribute('data-status', 'error');
      // The card must carry the error styling class (sets border-color: var(--color-danger)).
      const errorClass = toolCardStyles.cardError;
      if (!errorClass) throw new Error('cardError CSS module class missing');
      expect(card).toHaveClass(errorClass);
    });
  });

  // B5: retry flow tests
  describe('B5 retry flow', () => {
    it('retry rebuilds the agent turn from the last user message', async () => {
      // Seed a completed two-message turn: user -> assistant
      useStore.setState((s) => ({
        chat: {
          ...s.chat,
          _nextSeq: 2,
          messages: [
            { id: 'u1', role: 'user', content: 'attack the goblin', sequenceIndex: 0 },
            { id: 'a1', role: 'assistant', content: 'You strike!', sequenceIndex: 1 },
          ],
        },
      }));

      streamAgentTurnMock.mockImplementation(async (opts: AgentTurnOptions) => {
        opts.onTextDelta('You strike again!');
        opts.onAgentDone(1);
      });

      render(<ChatPanel />);

      // The assistant bubble's Retry button must be present (isNarrator = finalized assistant)
      const retryBtn = screen.getByLabelText(/retry/i);
      expect(retryBtn).toBeInTheDocument();

      fireEvent.click(retryBtn);

      await waitFor(() => {
        expect(streamAgentTurnMock).toHaveBeenCalledTimes(1);
      });
      // The agent was called with the original user message text
      expect(streamAgentTurnMock.mock.calls[0]?.[0].playerMessage).toBe('attack the goblin');

      await waitFor(() => {
        // The new assistant reply must be in the DOM
        expect(screen.getByText('You strike again!')).toBeInTheDocument();
      });

      // There must still be exactly one user message (no duplicate)
      const state = useStore.getState().chat;
      const userMsgs = state.messages.filter((m) => m.role === 'user');
      expect(userMsgs).toHaveLength(1);
      expect(userMsgs[0]).toMatchObject({ content: 'attack the goblin' });
    });

    it('"Retrying..." indicator is visible during replay and gone once settled', async () => {
      // Seed a completed turn so the Retry button is rendered.
      useStore.setState((s) => ({
        chat: {
          ...s.chat,
          _nextSeq: 2,
          messages: [
            { id: 'u1', role: 'user', content: 'search the room', sequenceIndex: 0 },
            { id: 'a1', role: 'assistant', content: 'You find nothing.', sequenceIndex: 1 },
          ],
        },
      }));

      // Hold the stream open until we manually resolve.
      let resolveStream!: () => void;
      const streamPending = new Promise<void>((resolve) => {
        resolveStream = resolve;
      });
      streamAgentTurnMock.mockImplementation(async (opts: AgentTurnOptions) => {
        await streamPending;
        opts.onTextDelta('You find a key!');
        opts.onAgentDone(1);
      });

      render(<ChatPanel />);

      const retryBtn = screen.getByLabelText(/retry/i);
      fireEvent.click(retryBtn);

      // The retrying indicator must appear immediately (before the stream settles).
      expect(screen.getByTestId('retrying-indicator')).toBeInTheDocument();

      // Let the stream complete.
      resolveStream();

      await waitFor(() => {
        expect(screen.queryByTestId('retrying-indicator')).not.toBeInTheDocument();
      });
    });

    it('retry is a no-op while a turn is already streaming', async () => {
      useStore.setState((s) => ({
        chat: {
          ...s.chat,
          _nextSeq: 2,
          messages: [
            { id: 'u1', role: 'user', content: 'first message', sequenceIndex: 0 },
            { id: 'a1', role: 'assistant', content: 'first reply', sequenceIndex: 1 },
          ],
          isStreaming: true,
        },
      }));

      render(<ChatPanel />);

      // With isStreaming=true the Retry button must be disabled so clicking it is a no-op
      const retryBtn = screen.getByLabelText(/retry/i);
      expect(retryBtn).toBeDisabled();

      fireEvent.click(retryBtn);

      // streamAgentTurn must not be called at all
      expect(streamAgentTurnMock).not.toHaveBeenCalled();
    });

    it('retry on intermediate assistant message rewinds to that turn and replays', async () => {
      // Seed a 4-message conversation: user1 -> assistant1 -> user2 -> assistant2
      useStore.setState((s) => ({
        chat: {
          ...s.chat,
          _nextSeq: 4,
          messages: [
            { id: 'u1', role: 'user', content: 'first action', sequenceIndex: 0 },
            { id: 'a1', role: 'assistant', content: 'first reply', sequenceIndex: 1 },
            { id: 'u2', role: 'user', content: 'second action', sequenceIndex: 2 },
            { id: 'a2', role: 'assistant', content: 'second reply', sequenceIndex: 3 },
          ],
        },
      }));

      streamAgentTurnMock.mockImplementation(async (opts: AgentTurnOptions) => {
        opts.onTextDelta('replayed reply');
        opts.onAgentDone(1);
      });

      render(<ChatPanel />);

      // Find the Retry button on the first assistant bubble (a1 = "first reply")
      // There are two assistant bubbles; getAllByLabelText returns them in DOM order.
      const retryBtns = screen.getAllByLabelText(/retry/i);
      // First button belongs to a1 (the intermediate message)
      const firstRetryBtn = retryBtns[0];
      if (!firstRetryBtn) throw new Error('Expected at least one retry button');
      fireEvent.click(firstRetryBtn);

      await waitFor(() => {
        expect(streamAgentTurnMock).toHaveBeenCalledTimes(1);
      });

      // send must have been called with the user message that preceded a1
      expect(streamAgentTurnMock.mock.calls[0]?.[0].playerMessage).toBe('first action');

      await waitFor(() => {
        expect(screen.getByText('replayed reply')).toBeInTheDocument();
      });

      const state = useStore.getState().chat;
      // user2 and a2 are gone; only user1 (re-appended by send) and the new assistant reply remain
      const userMsgs = state.messages.filter((m) => m.role === 'user');
      expect(userMsgs).toHaveLength(1);
      expect(userMsgs[0]).toMatchObject({ content: 'first action' });

      // The trailing messages (second action / second reply) must be gone
      expect(state.messages.find((m) => m.content === 'second action')).toBeUndefined();
      expect(state.messages.find((m) => m.content === 'second reply')).toBeUndefined();
    });
  });
});
