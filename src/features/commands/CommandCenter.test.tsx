import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import { Modal } from '../../ui/Modal';
import { CommandCenter } from './CommandCenter';
import type { AppCommand } from './model';

beforeEach(async () => {
  await i18n.changeLanguage('en');
});
describe('command center', () => {
  it('searches and executes an enabled command once, keeping focus restoration scoped', async () => {
    const run = vi.fn();
    const user = userEvent.setup();
    render(
      <CommandCenter commands={[{ id: 'hero', label: 'Open hero sheet', enabled: true, run }]} />,
    );
    const opener = screen.getByRole('button', { name: 'Commands' });
    await user.click(opener);
    await user.type(screen.getByRole('textbox', { name: 'Find a command' }), 'hero');
    await user.keyboard('{Enter}');
    await waitFor(() => expect(run).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(opener).toHaveFocus();
  });
  it('deduplicates held/in-flight saves and rejects latest disabled state before dispatch', async () => {
    let resolve: (() => void) | undefined;
    const run = vi.fn(
      () =>
        new Promise<void>((done) => {
          resolve = done;
        }),
    );
    const command: AppCommand = {
      id: 'save',
      label: 'Save',
      enabled: true,
      shortcut: { key: 's', primary: true, allowEditing: true },
      run,
    };
    const view = render(<CommandCenter commands={[command]} />);
    fireEvent.keyDown(window, { key: 's', ctrlKey: true });
    await waitFor(() => expect(run).toHaveBeenCalledTimes(1));
    fireEvent.keyDown(window, { key: 's', ctrlKey: true });
    fireEvent.keyDown(window, { key: 's', ctrlKey: true, repeat: true });
    expect(run).toHaveBeenCalledTimes(1);
    resolve?.();
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    view.rerender(<CommandCenter commands={[{ ...command, enabled: false }]} />);
    fireEvent.keyDown(window, { key: 's', ctrlKey: true });
    await new Promise((done) => window.setTimeout(done, 10));
    expect(run).toHaveBeenCalledTimes(1);
  });
  it('does not fire a queued command after scope unmount', async () => {
    const run = vi.fn();
    const view = render(
      <CommandCenter
        commands={[
          { id: 'save', label: 'Save', enabled: true, shortcut: { key: 's', primary: true }, run },
        ]}
      />,
    );
    fireEvent.keyDown(window, { key: 's', ctrlKey: true });
    view.unmount();
    await new Promise((done) => window.setTimeout(done, 10));
    expect(run).not.toHaveBeenCalled();
  });
  it('respects input and IME, while Ctrl+K opens a single palette', async () => {
    const run = vi.fn();
    render(
      <>
        <textarea aria-label="Draft" />
        <CommandCenter
          commands={[
            {
              id: 'journal',
              label: 'Journal',
              enabled: true,
              shortcut: { key: 'j', primary: true, shift: true },
              run,
            },
          ]}
        />
      </>,
    );
    const draft = screen.getByRole('textbox', { name: 'Draft' });
    draft.focus();
    fireEvent.keyDown(draft, { key: 'j', ctrlKey: true, shiftKey: true });
    fireEvent.keyDown(draft, { key: 'k', ctrlKey: true, isComposing: true });
    expect(run).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).toBeNull();
    fireEvent.keyDown(draft, { key: 'k', ctrlKey: true });
    fireEvent.keyDown(window, { key: 'k', ctrlKey: true, repeat: true });
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
  });
  it('Escape closes only the palette above another dialog', async () => {
    const parentClosed = vi.fn();
    function Nested() {
      const [open, setOpen] = useState(true);
      return (
        <>
          <Modal
            open={open}
            title="Parent"
            onClose={() => {
              parentClosed();
              setOpen(false);
            }}
          >
            <button type="button">Parent action</button>
          </Modal>
          <CommandCenter commands={[]} />
        </>
      );
    }
    render(<Nested />);
    fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(screen.getAllByRole('dialog')).toHaveLength(2);
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
    expect(parentClosed).not.toHaveBeenCalled();
  });
});
