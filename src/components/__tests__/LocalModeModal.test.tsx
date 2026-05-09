import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { LocalModeModal } from '../LocalModeModal';

describe('LocalModeModal runtime controls', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('shows an error chip when starting the runtime fails', async () => {
    globalThis.fetch = vi.fn(async () => {
      throw new Error('network down');
    }) as unknown as typeof fetch;

    const user = userEvent.setup();
    render(<LocalModeModal open={true} onClose={vi.fn()} />);

    await user.click(screen.getByRole('button', { name: /start runtimes/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/failed to start/i);
    });
  });

  it('keeps the start button at data-status=idle after a successful response', async () => {
    globalThis.fetch = vi.fn(
      async () => new Response(null, { status: 200 }) as unknown as Response,
    ) as unknown as typeof fetch;

    const user = userEvent.setup();
    render(<LocalModeModal open={true} onClose={vi.fn()} />);

    const button = screen.getByRole('button', { name: /start runtimes/i });
    await user.click(button);

    await waitFor(() => {
      expect(button).toHaveAttribute('data-status', 'idle');
    });
    expect(screen.queryByRole('alert')).toBeNull();
  });
});
