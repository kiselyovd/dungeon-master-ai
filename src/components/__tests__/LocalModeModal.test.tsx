import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setBackendPortForTesting } from '../../api/client';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { LocalModeModal } from '../LocalModeModal';

vi.mock('../../api/localLlm', () => ({
  fetchLocalLlmManifest: vi.fn(async () => ({
    system: [
      {
        id: 'qwen3.5-4b',
        display_name: 'Qwen3.5-4B',
        hf_repo: 'fixture/model',
        hf_filename: 'model.gguf',
        arch: 'qwen3',
        quant: 'q4',
        size_gb: 3,
        license: 'apache-2.0',
      },
    ],
    user: [],
    installed_ids: ['qwen3.5-4b'],
    download_states: {},
  })),
}));
const TEST_PORT = 49999;

describe('LocalModeModal runtime controls', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    setBackendPortForTesting(TEST_PORT);
    useStore.setState(useStore.getInitialState());
    useStore.getState().localMode.selectModel('qwen3_5_4b');
  });

  afterEach(() => {
    setBackendPortForTesting(null);
    globalThis.fetch = originalFetch;
  });

  it('shows an error chip when starting the runtime fails', async () => {
    globalThis.fetch = vi.fn(async () => {
      throw new Error('network down');
    }) as unknown as typeof fetch;

    const user = userEvent.setup();
    render(<LocalModeModal open={true} onClose={vi.fn()} />);

    const start = screen.getByRole('button', { name: /start model/i });
    await waitFor(() => expect(start).toBeEnabled());
    await user.click(start);

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/could not start/i);
    });
  });

  it('keeps the start button at data-status=idle after a successful response', async () => {
    globalThis.fetch = vi.fn(
      async () => new Response(null, { status: 200 }) as unknown as Response,
    ) as unknown as typeof fetch;

    const user = userEvent.setup();
    render(<LocalModeModal open={true} onClose={vi.fn()} />);

    const button = screen.getByRole('button', { name: /start model/i });
    await waitFor(() => expect(button).toBeEnabled());
    await user.click(button);

    await waitFor(() => {
      expect(button).toHaveAttribute('data-status', 'idle');
    });
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('resets error state when the modal is closed before the reset timer fires', async () => {
    globalThis.fetch = vi.fn(async () => {
      throw new Error('network down');
    }) as unknown as typeof fetch;

    const user = userEvent.setup();
    const { rerender } = render(<LocalModeModal open={true} onClose={vi.fn()} />);

    await user.click(screen.getByRole('button', { name: /start model/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/could not start/i);
    });

    // Close the modal while the reset timer is still pending.
    rerender(<LocalModeModal open={false} onClose={vi.fn()} />);
    expect(screen.queryByRole('alert')).toBeNull();

    // Reopen the modal: the error chip must not flash back from stale state.
    rerender(<LocalModeModal open={true} onClose={vi.fn()} />);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByRole('button', { name: /start model/i })).toHaveAttribute(
      'data-status',
      'idle',
    );
  });
});
