import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { type AgentTurnOptions, streamAgentTurn } from '../../api/agent';
import { fetchSessionMessages } from '../../api/chat';
import { ChatError } from '../../api/errors';
import { useStore } from '../../state/useStore';
import { ChatPanel } from '../ChatPanel';
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
});
