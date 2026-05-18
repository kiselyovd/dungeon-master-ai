import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ReasoningPill } from '../ReasoningPill';

// Mock i18next to return the key as-is for the few labels we use.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe('ReasoningPill', () => {
  it('renders nothing when no thinkingText and not streaming', () => {
    const { container } = render(<ReasoningPill thinkingText="" />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders pill collapsed by default', () => {
    render(<ReasoningPill thinkingText="hello" />);
    expect(screen.getByRole('button')).toBeInTheDocument();
    expect(screen.queryByText('hello')).not.toBeInTheDocument();
  });

  it('expands on click to show thinking text', () => {
    render(<ReasoningPill thinkingText="hello" />);
    fireEvent.click(screen.getByRole('button'));
    expect(screen.getByText('hello')).toBeInTheDocument();
  });

  it('collapses on second click', () => {
    render(<ReasoningPill thinkingText="x" />);
    const btn = screen.getByRole('button');
    fireEvent.click(btn);
    fireEvent.click(btn);
    expect(screen.queryByText('x')).not.toBeInTheDocument();
  });

  it('shows streaming label when streaming=true even with empty text', () => {
    render(<ReasoningPill thinkingText="" streaming />);
    expect(screen.getByText('reasoning_streaming_label')).toBeInTheDocument();
  });
});
