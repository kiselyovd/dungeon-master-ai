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

  // B3: markdown rendering tests
  it('renders h1 markdown as an h1 heading element in finalized assistant bubbles', () => {
    render(<MessageBubble chatRole="assistant"># The Dragon Awakens</MessageBubble>);
    const bubble = screen.getByTestId('bubble');
    expect(bubble.querySelector('h1')).not.toBeNull();
    expect(bubble.querySelector('h1')?.textContent).toBe('The Dragon Awakens');
  });

  it('streaming bubbles bypass markdown and keep raw text', () => {
    render(
      <MessageBubble chatRole="assistant" streaming>
        # Partial heading
      </MessageBubble>,
    );
    const bubble = screen.getByTestId('bubble');
    // No h1 rendered - raw text only
    expect(bubble.querySelector('h1')).toBeNull();
    expect(bubble.textContent).toContain('# Partial heading');
  });

  it('inline code in finalized assistant bubble gets the inlineCode class', () => {
    render(<MessageBubble chatRole="assistant">Use `fireball` spell.</MessageBubble>);
    const bubble = screen.getByTestId('bubble');
    const code = bubble.querySelector('code');
    expect(code).not.toBeNull();
    // Non-null assertion justified: the expect above guards it
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    expect(code!.className).toContain('inlineCode');
  });

  it('strong text in finalized assistant bubble gets the strong class', () => {
    render(<MessageBubble chatRole="assistant">**Critical hit!**</MessageBubble>);
    const bubble = screen.getByTestId('bubble');
    const strong = bubble.querySelector('strong');
    expect(strong).not.toBeNull();
    // Non-null assertion justified: the expect above guards it
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    expect(strong!.className).toContain('strong');
  });

  it('drop-cap targets the first paragraph inside markdownBody for narrator bubbles', () => {
    render(<MessageBubble chatRole="assistant">The tavern is silent.</MessageBubble>);
    const bubble = screen.getByTestId('bubble');
    // Finalized assistant bubble has data-narrator="true"
    expect(bubble.getAttribute('data-narrator')).toBe('true');
    // The markdown body wrapper must be present - query by a partial class match
    // since CSS modules hash the class name (contains "markdownBody")
    const markdownBody = bubble.querySelector('[class*="markdownBody"]');
    expect(markdownBody).not.toBeNull();
    // The first child of markdownBody must be a <p> so the CSS drop-cap rule applies
    expect(markdownBody?.firstElementChild?.tagName.toLowerCase()).toBe('p');
  });

  it('fenced code blocks render inside a <pre> with fencedCode styling', () => {
    // Language-hinted fence: react-markdown gives the <code> a "language-*" class.
    const { unmount } = render(
      <MessageBubble chatRole="assistant">{'```js\nconsole.log("hello");\n```'}</MessageBubble>,
    );
    const bubble = screen.getByTestId('bubble');
    const pre = bubble.querySelector('pre');
    expect(pre).not.toBeNull();
    // Non-null assertion justified: the expect above guards it
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    const codeHinted = pre!.querySelector('code');
    expect(codeHinted).not.toBeNull();
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    expect(codeHinted!.className).toContain('fencedCode');
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    expect(codeHinted!.className).not.toContain('inlineCode');
    unmount();

    // Unlabeled fence: no language hint, so react-markdown gives className=undefined.
    // This is the case the original code misclassified as inline.
    render(<MessageBubble chatRole="assistant">{'```\nsome block\n```'}</MessageBubble>);
    const bubble2 = screen.getByTestId('bubble');
    const pre2 = bubble2.querySelector('pre');
    expect(pre2).not.toBeNull();
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    const codeUnlabeled = pre2!.querySelector('code');
    expect(codeUnlabeled).not.toBeNull();
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    expect(codeUnlabeled!.className).toContain('fencedCode');
    // biome-ignore lint/style/noNonNullAssertion: guarded by expect above
    expect(codeUnlabeled!.className).not.toContain('inlineCode');
  });
});
