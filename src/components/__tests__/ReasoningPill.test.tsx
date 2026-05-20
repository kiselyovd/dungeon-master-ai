import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ReasoningPill } from '../ReasoningPill';

// Mock i18next. Returns translations with simple {{var}} interpolation so
// tests can assert on the rendered, human-readable string (e.g. "Thinking...
// 50 tok"). The mock t() receives the base plural key + { count } and falls
// through to the _other template for any count != 1.
const REASONING_DICT: Record<string, string> = {
  // plural forms: _one / _other
  'reasoning.thinking_with_tokens_one': 'Thinking... 1 tok',
  'reasoning.thinking_with_tokens_other': 'Thinking... {{count}} tok',
  'reasoning.collapsed_label_one': 'Reasoning 1 tok',
  'reasoning.collapsed_label_other': 'Reasoning {{count}} tok',
  'reasoning.expand': 'Show reasoning',
  'reasoning.collapse': 'Hide reasoning',
};

function interpolate(template: string, vars: Record<string, unknown>): string {
  return template.replace(/\{\{(\w+)\}\}/g, (_match, name: string) => {
    const v = vars[name];
    return v === undefined || v === null ? '' : String(v);
  });
}

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, vars?: Record<string, unknown>) => {
      // Resolve plural suffix: count=1 -> _one, everything else -> _other.
      // Falls back to exact key if no plural variant is found.
      const count = vars && typeof vars.count === 'number' ? vars.count : undefined;
      const suffix = count === 1 ? '_one' : count !== undefined ? '_other' : undefined;
      const template =
        (suffix !== undefined ? REASONING_DICT[`${key}${suffix}`] : undefined) ??
        REASONING_DICT[key];
      if (template === undefined) return key;
      return vars ? interpolate(template, vars) : template;
    },
  }),
}));

describe('ReasoningPill', () => {
  it('renders pill collapsed by default', () => {
    render(<ReasoningPill text="hello" isStreaming={false} />);
    expect(screen.getByRole('button')).toBeInTheDocument();
    expect(screen.queryByText('hello')).not.toBeInTheDocument();
  });

  it('expands on click to show thinking text', () => {
    render(<ReasoningPill text="hello" isStreaming={false} />);
    fireEvent.click(screen.getByRole('button'));
    expect(screen.getByText('hello')).toBeInTheDocument();
  });

  it('collapses on second click', () => {
    render(<ReasoningPill text="x" isStreaming={false} />);
    const btn = screen.getByRole('button');
    fireEvent.click(btn);
    fireEvent.click(btn);
    expect(screen.queryByText('x')).not.toBeInTheDocument();
  });

  it('shows streaming label when isStreaming=true even with empty text', () => {
    render(<ReasoningPill text="" isStreaming={true} />);
    expect(screen.getByText(/Thinking\.\.\./i)).toBeInTheDocument();
  });
});

describe('ReasoningPill B6 polish', () => {
  it('shows Thinking... when isStreaming=true and no reasoning text has arrived yet (streamingReasoning === null)', () => {
    // isStreaming prop should be true when streaming AND reasoning text is null
    render(<ReasoningPill text="" isStreaming={true} />);
    expect(screen.getByTestId('reasoning-thinking')).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('shows collapsed token-count label once reasoning text is present (isStreaming=false)', () => {
    // Once the first reasoning delta arrives, isStreaming=false flips to the
    // collapsed summary button; chevron reflects collapsed state (aria-expanded=false)
    render(<ReasoningPill text="step by step analysis" isStreaming={false} />);
    const btn = screen.getByRole('button', { name: /Reasoning .* tok/i });
    expect(btn).toBeInTheDocument();
    expect(btn).toHaveAttribute('aria-expanded', 'false');
    // The pill body must be hidden until expanded
    expect(screen.queryByTestId('reasoning-body')).not.toBeInTheDocument();
    // Clicking expands it
    fireEvent.click(btn);
    expect(btn).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByTestId('reasoning-body')).toBeInTheDocument();
  });
});

describe('ReasoningPill M9 polish', () => {
  it('renders collapsed-by-default summary after stream completes', () => {
    render(
      <ReasoningPill
        text="The user is asking for a calculation. Let me think step by step."
        isStreaming={false}
      />,
    );
    expect(screen.getByRole('button', { name: /Reasoning .* tok/i })).toBeInTheDocument();
    expect(screen.queryByTestId('reasoning-body')).not.toBeInTheDocument();
  });

  it('expands on summary click and toggles aria-expanded', () => {
    render(<ReasoningPill text="hidden body content for the test" isStreaming={false} />);
    const btn = screen.getByRole('button', { name: /Reasoning .* tok/i });
    expect(btn).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(btn);
    expect(btn).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByTestId('reasoning-body')).toHaveTextContent(
      'hidden body content for the test',
    );
  });

  it('shows live "Thinking... N tok" during streaming', () => {
    const text = 'a'.repeat(200);
    render(<ReasoningPill text={text} isStreaming={true} />);
    expect(screen.getByText(/Thinking\.\.\. 50 tok/i)).toBeInTheDocument();
  });

  it('updates token count as text grows', () => {
    const { rerender } = render(<ReasoningPill text={'x'.repeat(40)} isStreaming={true} />);
    expect(screen.getByText(/Thinking\.\.\. 10 tok/i)).toBeInTheDocument();
    rerender(<ReasoningPill text={'x'.repeat(400)} isStreaming={true} />);
    expect(screen.getByText(/Thinking\.\.\. 100 tok/i)).toBeInTheDocument();
  });

  it('respects prefers-reduced-motion by skipping the transition class', () => {
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: vi.fn().mockImplementation((query: string) => ({
        matches: query.includes('reduce'),
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });
    const { container } = render(<ReasoningPill text="some thinking" isStreaming={false} />);
    expect(container.querySelector('[data-reduced-motion="true"]')).toBeInTheDocument();
  });
});
