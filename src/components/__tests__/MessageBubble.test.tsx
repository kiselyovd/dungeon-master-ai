import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { MessagePart } from '../../state/chat';
import { MessageBubble } from '../MessageBubble';
import '../../i18n';

describe('MessageBubble', () => {
  it('renders children when no parts are provided', () => {
    render(<MessageBubble chatRole="assistant">hello</MessageBubble>);
    expect(screen.getByTestId('bubble').textContent).toBe('hello');
  });

  it('renders text and image parts in order', () => {
    const parts: MessagePart[] = [
      { type: 'text', text: 'before' },
      { type: 'image', mime: 'image/png', data_b64: 'aGk=', name: 'p.png' },
      { type: 'text', text: 'after' },
    ];
    render(
      <MessageBubble chatRole="user" parts={parts}>
        before
      </MessageBubble>,
    );
    const bubble = screen.getByTestId('bubble');
    const children = Array.from(bubble.children);
    expect(children).toHaveLength(3);
    expect(children[0]?.textContent).toBe('before');
    // Image is wrapped in a <button> so the click target is keyboard-reachable.
    expect(children[1]?.tagName).toBe('BUTTON');
    expect(children[1]?.querySelector('img')).not.toBeNull();
    expect(children[2]?.textContent).toBe('after');
  });

  it('opens the lightbox when an image is clicked', () => {
    const parts: MessagePart[] = [
      { type: 'image', mime: 'image/png', data_b64: 'aGk=', name: 'p.png' },
    ];
    render(
      <MessageBubble chatRole="user" parts={parts}>
        x
      </MessageBubble>,
    );
    fireEvent.click(screen.getByRole('img'));
    expect(screen.getByTestId('lightbox-image')).toBeInTheDocument();
  });
});
