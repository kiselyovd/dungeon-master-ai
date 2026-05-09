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

  it('exposes the bubble as role=article with an aria-label combining role + text preview', () => {
    render(<MessageBubble chatRole="user">I draw my sword.</MessageBubble>);
    const article = screen.getByRole('article');
    expect(article).toBe(screen.getByTestId('bubble'));
    expect(article.getAttribute('aria-label')).toBe('User: I draw my sword.');
  });

  it('labels assistant bubbles distinctly from user bubbles', () => {
    render(<MessageBubble chatRole="assistant">A dragon roars in the dark.</MessageBubble>);
    const article = screen.getByRole('article');
    expect(article.getAttribute('aria-label')).toMatch(/^Assistant:/);
  });

  it('appends an image-count suffix when the message includes images', () => {
    const parts: MessagePart[] = [
      { type: 'text', text: 'check this map' },
      { type: 'image', mime: 'image/png', data_b64: 'aGk=', name: 'a.png' },
      { type: 'image', mime: 'image/png', data_b64: 'aGk=', name: 'b.png' },
    ];
    render(
      <MessageBubble chatRole="user" parts={parts}>
        check this map
      </MessageBubble>,
    );
    const article = screen.getByRole('article');
    expect(article.getAttribute('aria-label')).toMatch(/check this map/);
    expect(article.getAttribute('aria-label')).toMatch(/2 images attached/);
  });

  it('marks the bubble aria-busy while streaming', () => {
    render(
      <MessageBubble chatRole="assistant" streaming>
        partial...
      </MessageBubble>,
    );
    expect(screen.getByRole('article').getAttribute('aria-busy')).toBe('true');
  });
});
