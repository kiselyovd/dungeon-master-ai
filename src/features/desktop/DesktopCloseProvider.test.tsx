import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import {
  DesktopCloseProvider,
  useDesktopClosePolicy,
  useDesktopCloseRequest,
} from './DesktopCloseProvider';
import type { ClosePolicy } from './model/closePolicy';

const bridge = vi.hoisted(() => ({
  exit: vi.fn(async () => {}),
  cleanup: vi.fn(),
  request: undefined as undefined | (() => void),
}));
vi.mock('../../api/desktop', () => ({
  confirmDesktopExit: bridge.exit,
  listenForDesktopClose: vi.fn(async (request: () => void) => {
    bridge.request = request;
    return bridge.cleanup;
  }),
}));
function Registered({ policy }: { policy: ClosePolicy }) {
  useDesktopClosePolicy(policy);
  const close = useDesktopCloseRequest();
  return (
    <button type="button" onClick={close}>
      Request close
    </button>
  );
}
function App({ policy }: { policy: ClosePolicy }) {
  return (
    <DesktopCloseProvider>
      <Registered policy={policy} />
    </DesktopCloseProvider>
  );
}
beforeEach(async () => {
  vi.clearAllMocks();
  bridge.exit.mockResolvedValue(undefined);
  bridge.request = undefined;
  await i18n.changeLanguage('en');
});
describe('desktop close guard', () => {
  it('requires a second explicit choice before exiting with unconfirmed departure', async () => {
    const preserveAndExit = vi.fn(async () => {});
    const prepare = vi.fn(async () => ({
      kind: 'unconfirmed_departure' as const,
      preserveAndExit,
    }));
    const user = userEvent.setup();
    render(<App policy={{ scope: 'room', kind: 'guest', pending: true, prepare }} />);
    await user.click(screen.getByRole('button', { name: 'Request close' }));
    await user.click(screen.getByRole('button', { name: 'Close application' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Departure was not confirmed');
    expect(preserveAndExit).not.toHaveBeenCalled();
    expect(bridge.exit).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Close and keep recovery' }));
    await waitFor(() => expect(bridge.exit).toHaveBeenCalledTimes(1));
    expect(preserveAndExit).toHaveBeenCalledTimes(1);
  });
  it('clears a canceled departure fallback before entering another scope', async () => {
    const oldPreserve = vi.fn(async () => {});
    const user = userEvent.setup();
    const view = render(
      <App
        policy={{
          scope: 'old',
          kind: 'guest',
          pending: false,
          prepare: async () => ({ kind: 'unconfirmed_departure', preserveAndExit: oldPreserve }),
        }}
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Request close' }));
    await user.click(screen.getByRole('button', { name: 'Close application' }));
    await screen.findByRole('alert');
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    view.rerender(
      <App policy={{ scope: 'new', kind: 'host', pending: false, prepare: async () => {} }} />,
    );
    await user.click(screen.getByRole('button', { name: 'Request close' }));
    expect(screen.queryByRole('button', { name: 'Close and keep recovery' })).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Close application' }));
    await waitFor(() => expect(bridge.exit).toHaveBeenCalledTimes(1));
    expect(oldPreserve).not.toHaveBeenCalled();
  });
  it.each([
    'host',
    'guest',
  ] as const)('explains %s departure, defaults to Cancel and deduplicates native requests', async (kind) => {
    const prepare = vi.fn(async () => {});
    const user = userEvent.setup();
    render(<App policy={{ scope: 'room', kind, pending: false, prepare }} />);
    await waitFor(() => expect(bridge.request).toBeTypeOf('function'));
    fireEvent.click(screen.getByRole('button', { name: 'Request close' }));
    bridge.request?.();
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
    expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
    expect(
      screen.getByText(
        kind === 'host'
          ? 'This stops your hosted room and disconnects its players.'
          : 'This leaves your room and closes the application.',
      ),
    ).toBeVisible();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(prepare).not.toHaveBeenCalled();
    expect(bridge.exit).not.toHaveBeenCalled();
  });
  it('waits for canonical departure and disk barriers before one confirmed exit', async () => {
    let release: (() => void) | undefined;
    const prepare = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          release = resolve;
        }),
    );
    const user = userEvent.setup();
    render(<App policy={{ scope: 'room', kind: 'host', pending: true, prepare }} />);
    await user.click(screen.getByRole('button', { name: 'Request close' }));
    expect(screen.getByText(/An operation is still pending/)).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Close application' }));
    bridge.request?.();
    expect(screen.getByRole('button', { name: 'Closing…' })).toBeDisabled();
    await user.keyboard('{Escape}');
    expect(screen.getByRole('dialog')).toBeVisible();
    expect(prepare).toHaveBeenCalledTimes(1);
    expect(bridge.exit).not.toHaveBeenCalled();
    release?.();
    await waitFor(() => expect(bridge.exit).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });
  it('keeps the app open and surfaces a safe failure when departure is unconfirmed', async () => {
    const prepare = vi.fn(async () => {
      throw Error('private-fixture-payload');
    });
    const warn = vi.spyOn(console, 'warn');
    const user = userEvent.setup();
    render(<App policy={{ scope: 'room', kind: 'guest', pending: false, prepare }} />);
    await user.click(screen.getByRole('button', { name: 'Request close' }));
    await user.click(screen.getByRole('button', { name: 'Close application' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('The application remains open');
    expect(screen.queryByText('private-fixture-payload')).toBeNull();
    expect(bridge.exit).not.toHaveBeenCalled();
    expect(warn).toHaveBeenCalledWith('[desktop.close]', { code: 'desktop_close_failed' });
  });
  it('refuses an old confirmation after changing campaign or connection', async () => {
    const prepare = vi.fn(async () => {});
    const user = userEvent.setup();
    const policy: ClosePolicy = { scope: 'old', kind: 'solo', pending: true, prepare };
    const view = render(<App policy={policy} />);
    await user.click(screen.getByRole('button', { name: 'Request close' }));
    view.rerender(<App policy={{ ...policy, scope: 'new' }} />);
    await user.click(screen.getByRole('button', { name: 'Close application' }));
    expect(await screen.findByRole('alert')).toBeVisible();
    expect(prepare).not.toHaveBeenCalled();
    expect(bridge.exit).not.toHaveBeenCalled();
  });
  it('closes idle solo after preparing state without requiring a confirmation click', async () => {
    const prepare = vi.fn(async () => {});
    render(<App policy={{ scope: 'solo', kind: 'solo', pending: false, prepare }} />);
    fireEvent.click(screen.getByRole('button', { name: 'Request close' }));
    await waitFor(() => expect(bridge.exit).toHaveBeenCalledTimes(1));
    expect(prepare).toHaveBeenCalledTimes(1);
  });
  it('removes a native listener which resolves after unmount', async () => {
    const { listenForDesktopClose } = await import('../../api/desktop');
    let resolve: ((cleanup: () => void) => void) | undefined;
    vi.mocked(listenForDesktopClose).mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const view = render(
      <App policy={{ scope: 'solo', kind: 'solo', pending: false, prepare: async () => {} }} />,
    );
    view.unmount();
    resolve?.(bridge.cleanup);
    await waitFor(() => expect(bridge.cleanup).toHaveBeenCalledTimes(1));
  });
});
