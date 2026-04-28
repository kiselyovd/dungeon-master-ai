import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ChatPanel } from '../ChatPanel';
import { useStore } from '../../state/useStore';
import '../../i18n';

describe('ChatPanel', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  it('renders empty state with placeholder when no messages', () => {
    render(<ChatPanel />);
    const input = screen.getByPlaceholderText(/Type a message/i);
    expect(input).toBeInTheDocument();
  });

  it('renders existing messages from store', () => {
    useStore.setState((s) => ({
      chat: {
        ...s.chat,
        messages: [
          { role: 'user', content: 'hello' },
          { role: 'assistant', content: 'hi there' },
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
    const input = screen.getByPlaceholderText(/Type a message/i);
    fireEvent.change(input, { target: { value: 'test' } });
    const button = screen.getByRole('button', { name: /Send/i });
    expect(button).toBeEnabled();
  });
});
