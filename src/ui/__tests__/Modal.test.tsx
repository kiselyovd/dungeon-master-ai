import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { Modal } from '../Modal';

function Harness({ onClose }: { onClose: () => void }) {
  const [open, setOpen] = useState(true);
  return (
    <Modal
      open={open}
      onClose={() => {
        onClose();
        setOpen(false);
      }}
      title="Test dialog"
    >
      <button type="button">first</button>
      <input aria-label="middle" />
      <button type="button">last</button>
    </Modal>
  );
}

describe('Modal', () => {
  it('does not render when closed', () => {
    render(<Modal open={false} onClose={() => {}} title="hidden" />);
    expect(screen.queryByText(/hidden/)).not.toBeInTheDocument();
  });

  it('renders with role=dialog and aria-labelledby pointing at the title', () => {
    render(<Modal open onClose={() => {}} title="My Dialog" />);
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    const labelId = dialog.getAttribute('aria-labelledby');
    expect(labelId).toBeTruthy();
    const heading = screen.getByText('My Dialog');
    expect(heading).toHaveAttribute('id', labelId);
  });

  it('closes on Escape', () => {
    const onClose = vi.fn();
    render(<Harness onClose={onClose} />);
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });

  it('focuses the first focusable element on open', () => {
    render(<Harness onClose={() => {}} />);
    expect(screen.getByText('first')).toHaveFocus();
  });

  it('traps Tab cycling within the dialog', async () => {
    const user = userEvent.setup();
    render(<Harness onClose={() => {}} />);
    const first = screen.getByText('first');
    const middle = screen.getByLabelText('middle');
    const last = screen.getByText('last');

    expect(first).toHaveFocus();
    await user.tab();
    expect(middle).toHaveFocus();
    await user.tab();
    expect(last).toHaveFocus();
    await user.tab();
    // Tab from last wraps back to first.
    expect(first).toHaveFocus();
    await user.tab({ shift: true });
    expect(last).toHaveFocus();
  });

  it('restores focus to the previously-focused element on close', async () => {
    const user = userEvent.setup();
    function ParentHarness() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <button type="button" onClick={() => setOpen(true)}>
            opener
          </button>
          <Modal open={open} onClose={() => setOpen(false)} title="t">
            <button type="button">inside</button>
          </Modal>
        </>
      );
    }

    render(<ParentHarness />);
    const opener = screen.getByText('opener');
    opener.focus();
    expect(opener).toHaveFocus();

    await user.click(opener);
    expect(screen.getByText('inside')).toHaveFocus();

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(opener).toHaveFocus();
  });

  it('closes when clicking the backdrop', () => {
    const onClose = vi.fn();
    render(<Modal open onClose={onClose} title="x" />);
    const backdrop = screen.getByRole('presentation');
    fireEvent.mouseDown(backdrop);
    expect(onClose).toHaveBeenCalled();
  });
});
