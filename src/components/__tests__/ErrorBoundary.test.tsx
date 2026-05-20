import { fireEvent, render, screen } from '@testing-library/react';
import { Component, type ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useStore } from '../../state/useStore';
import { ErrorBoundary } from '../ErrorBoundary';

// A component that throws on demand
class Thrower extends Component<{ shouldThrow: boolean }> {
  override render() {
    if (this.props.shouldThrow) throw new Error('test-crash');
    return <div data-testid="child-ok">ok</div>;
  }
}

// Suppress React's console.error output for expected throws in tests
beforeEach(() => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
  useStore.setState(useStore.getInitialState());
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('ErrorBoundary - top level', () => {
  it('renders children normally when no error', () => {
    render(
      <ErrorBoundary level="top">
        <div data-testid="child-ok">ok</div>
      </ErrorBoundary>,
    );
    expect(screen.getByTestId('child-ok')).toBeTruthy();
  });

  it('shows top-level error card with Reload and Copy buttons when child throws', () => {
    render(
      <ErrorBoundary level="top">
        <Thrower shouldThrow={true} />
      </ErrorBoundary>,
    );
    expect(screen.getByTestId('error-boundary-top')).toBeTruthy();
    expect(screen.getByRole('button', { name: /reload/i })).toBeTruthy();
    expect(screen.getByRole('button', { name: /copy/i })).toBeTruthy();
  });

  it('copy crash report calls navigator.clipboard.writeText', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText },
      writable: true,
      configurable: true,
    });

    render(
      <ErrorBoundary level="top">
        <Thrower shouldThrow={true} />
      </ErrorBoundary>,
    );

    fireEvent.click(screen.getByRole('button', { name: /copy/i }));
    expect(writeText).toHaveBeenCalledOnce();
    const firstCall = writeText.mock.calls[0];
    expect(firstCall).toBeDefined();
    const payload = firstCall?.[0] as string;
    expect(payload).toContain('test-crash');
    expect(payload).toContain(__APP_VERSION__);
  });
});

describe('ErrorBoundary - section level', () => {
  it('shows section error card with Retry button when child throws', () => {
    render(
      <ErrorBoundary level="section">
        <Thrower shouldThrow={true} />
      </ErrorBoundary>,
    );
    expect(screen.getByTestId('error-boundary-section')).toBeTruthy();
    expect(screen.getByRole('button', { name: /retry/i })).toBeTruthy();
  });

  it('Retry button remounts children (clears error state)', () => {
    // We need a stateful wrapper that can flip shouldThrow off after mount
    let throwNow = true;

    function ThrowOnce({ children }: { children: ReactNode }) {
      if (throwNow) throw new Error('first-crash');
      return <>{children}</>;
    }

    render(
      <ErrorBoundary level="section">
        <ThrowOnce>
          <div data-testid="recovered">recovered</div>
        </ThrowOnce>
      </ErrorBoundary>,
    );

    // Should show the error card
    expect(screen.getByTestId('error-boundary-section')).toBeTruthy();

    // Now stop throwing so the remounted subtree works
    throwNow = false;

    fireEvent.click(screen.getByRole('button', { name: /retry/i }));

    // handleRetry calls setState which triggers React's own re-render;
    // no rerender() needed - verify the boundary's own retry mechanism directly
    expect(screen.getByTestId('recovered')).toBeTruthy();
  });
});
