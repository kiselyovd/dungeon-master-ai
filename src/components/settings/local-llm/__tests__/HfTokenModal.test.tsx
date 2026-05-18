import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../../../i18n';
import { HfTokenModal } from '../HfTokenModal';

vi.mock('../../../../api/hf', () => ({
  setToken: vi.fn(async (token: string) => ({
    connected: true,
    prefix: `${token.slice(0, 4)}...${token.slice(-4)}`,
  })),
}));

describe('HfTokenModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('saves token and calls onSaved', async () => {
    const onSaved = vi.fn();
    const onClose = vi.fn();
    render(<HfTokenModal open={true} onClose={onClose} onSaved={onSaved} />);
    fireEvent.change(screen.getByLabelText(/token/i), {
      target: { value: 'hf_abcdefghij1234' },
    });
    fireEvent.click(screen.getByRole('button', { name: /save/i }));
    await waitFor(() => {
      expect(onSaved).toHaveBeenCalled();
    });
  });

  it('cancels without saving', () => {
    const onClose = vi.fn();
    const onSaved = vi.fn();
    render(<HfTokenModal open={true} onClose={onClose} onSaved={onSaved} />);
    fireEvent.click(screen.getByRole('button', { name: /cancel/i }));
    expect(onClose).toHaveBeenCalled();
    expect(onSaved).not.toHaveBeenCalled();
  });

  it('does not render when closed', () => {
    render(
      <HfTokenModal
        open={false}
        onClose={() => {
          /* noop */
        }}
        onSaved={() => {
          /* noop */
        }}
      />,
    );
    expect(screen.queryByLabelText(/token/i)).not.toBeInTheDocument();
  });
});
