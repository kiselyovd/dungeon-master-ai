import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { TypingIndicator } from '../TypingIndicator';

describe('TypingIndicator', () => {
  it('renders three diamond glyphs', () => {
    const { container } = render(<TypingIndicator />);
    const diamonds = container.querySelectorAll('[data-diamond]');
    expect(diamonds).toHaveLength(3);
  });

  it('has aria-label for accessibility', () => {
    const { container } = render(<TypingIndicator />);
    const wrapper = container.querySelector('[aria-label]');
    expect(wrapper).toBeTruthy();
  });

  it('uses role=status so screen readers announce stream start', () => {
    const { container } = render(<TypingIndicator />);
    expect(container.querySelector('[role="status"]')).toBeTruthy();
  });
});
