import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../../../i18n';
import { HfTokenModal } from '../HfTokenModal';

const mockSetToken = vi.fn();

vi.mock('../../../../api/hf', () => ({
  setToken: (...args: unknown[]) => mockSetToken(...args),
}));

describe('HfTokenModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the token input', () => {
    render(<HfTokenModal open={true} onClose={vi.fn()} onSaved={vi.fn()} />);
    expect(screen.getByLabelText(/token \(hf_/i)).toBeInTheDocument();
  });

  it('successful save calls onSaved and onClose', async () => {
    mockSetToken.mockResolvedValue({ connected: true, prefix: 'hf_a...1234' });
    const onSaved = vi.fn();
    const onClose = vi.fn();
    render(<HfTokenModal open={true} onClose={onClose} onSaved={onSaved} />);
    fireEvent.change(screen.getByLabelText(/token \(hf_/i), {
      target: { value: 'hf_abcdefghij1234' },
    });
    fireEvent.click(screen.getByRole('button', { name: /save/i }));
    await waitFor(() => {
      expect(onSaved).toHaveBeenCalled();
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('failed save shows an error message', async () => {
    mockSetToken.mockRejectedValue(new Error('Invalid token'));
    const onSaved = vi.fn();
    const onClose = vi.fn();
    render(<HfTokenModal open={true} onClose={onClose} onSaved={onSaved} />);
    fireEvent.change(screen.getByLabelText(/token \(hf_/i), {
      target: { value: 'hf_badtoken' },
    });
    fireEvent.click(screen.getByRole('button', { name: /save/i }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(onSaved).not.toHaveBeenCalled();
  });

  it('Save button is disabled when the input is empty', () => {
    render(<HfTokenModal open={true} onClose={vi.fn()} onSaved={vi.fn()} />);
    expect(screen.getByRole('button', { name: /save/i })).toBeDisabled();
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
    expect(screen.queryByLabelText(/token \(hf_/i)).not.toBeInTheDocument();
  });

  it('resets input value when modal is closed and reopened', () => {
    const { rerender } = render(<HfTokenModal open={true} onClose={vi.fn()} onSaved={vi.fn()} />);
    fireEvent.change(screen.getByLabelText(/token \(hf_/i), {
      target: { value: 'hf_staletoken' },
    });
    rerender(<HfTokenModal open={false} onClose={vi.fn()} onSaved={vi.fn()} />);
    rerender(<HfTokenModal open={true} onClose={vi.fn()} onSaved={vi.fn()} />);
    expect(screen.getByLabelText(/token \(hf_/i)).toHaveValue('');
  });
});
