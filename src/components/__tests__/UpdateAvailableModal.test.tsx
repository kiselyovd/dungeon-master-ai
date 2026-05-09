import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { UpdateAvailableModal } from '../UpdateAvailableModal';

describe('UpdateAvailableModal', () => {
  it('renders changelog and version when shown', () => {
    render(
      <UpdateAvailableModal
        version="0.6.0"
        notes="bug fixes"
        onUpdate={vi.fn()}
        onLater={vi.fn()}
      />,
    );
    expect(screen.getByText(/0\.6\.0/)).toBeInTheDocument();
    expect(screen.getByText(/bug fixes/)).toBeInTheDocument();
  });

  it('calls onUpdate when Update Now clicked', () => {
    const onUpdate = vi.fn();
    render(<UpdateAvailableModal version="0.6.0" notes="" onUpdate={onUpdate} onLater={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: /update now/i }));
    expect(onUpdate).toHaveBeenCalled();
  });

  it('calls onLater when Later clicked', () => {
    const onLater = vi.fn();
    render(<UpdateAvailableModal version="0.6.0" notes="" onUpdate={vi.fn()} onLater={onLater} />);
    fireEvent.click(screen.getByRole('button', { name: /later/i }));
    expect(onLater).toHaveBeenCalled();
  });
});
