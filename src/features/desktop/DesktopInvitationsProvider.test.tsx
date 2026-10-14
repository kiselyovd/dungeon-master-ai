import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import type { InvitationEvent } from '../../api/invitations';
import i18n from '../../i18n';
import { DesktopInvitationsProvider, useInvitationTarget } from './DesktopInvitationsProvider';

const bridge = vi.hoisted(() => ({
  events: [] as InvitationEvent[],
  notify: undefined as (() => void) | undefined,
  cleanup: vi.fn(),
}));
vi.mock('../../api/invitations', () => ({
  takeInvitation: vi.fn(async () => bridge.events.shift() ?? null),
  listenForInvitations: vi.fn(async (notify: () => void) => {
    bridge.notify = notify;
    return bridge.cleanup;
  }),
}));
const event: InvitationEvent = {
  event_id: '00000000-0000-4000-8000-000000000002',
  preview: {
    address: '127.0.0.1:4123',
    room_id: '00000000-0000-4000-8000-000000000001',
    invitation: 'a'.repeat(64),
  },
};
const commit = vi.fn();
function Registered({
  scope = 'solo',
  blocked = false,
  prepare = async () => commit,
}: {
  scope?: string;
  blocked?: boolean;
  prepare?: () => Promise<() => void>;
}) {
  useInvitationTarget({ scope, blocked, priority: 1, prepare });
  return <textarea aria-label="Current draft" defaultValue="Untouched draft" />;
}
function App(props: Parameters<typeof Registered>[0]) {
  return (
    <DesktopInvitationsProvider>
      <Registered {...props} />
    </DesktopInvitationsProvider>
  );
}
beforeEach(async () => {
  bridge.events = [{ ...event }];
  bridge.notify = undefined;
  vi.clearAllMocks();
  await i18n.changeLanguage('en');
});
it('queues an early event and requires an explicit action, without rendering the token', async () => {
  const user = userEvent.setup();
  render(<App />);
  const dialog = await screen.findByRole('dialog', { name: 'Room invitation' });
  expect(dialog).toHaveTextContent(event.preview.room_id);
  expect(dialog.textContent).not.toContain(event.preview.invitation);
  expect(commit).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(screen.getByLabelText('Current draft')).toHaveValue('Untouched draft');
  expect(commit).not.toHaveBeenCalled();
});
it('deduplicates repeated activation while previewing', async () => {
  const user = userEvent.setup();
  bridge.events.push({ ...event, event_id: '00000000-0000-4000-8000-000000000003' });
  render(<App />);
  await screen.findByRole('dialog');
  await user.click(screen.getByRole('button', { name: 'Open join form' }));
  await waitFor(() => expect(commit).toHaveBeenCalledOnce());
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
});
it('blocks replacement of a connected room or pending operation', async () => {
  render(<App blocked />);
  await screen.findByRole('dialog');
  expect(screen.getByRole('button', { name: 'Open join form' })).toBeDisabled();
  expect(commit).not.toHaveBeenCalled();
});
it('refuses a stale destination after asynchronous draft preparation', async () => {
  const user = userEvent.setup();
  let ready: ((commit: () => void) => void) | undefined;
  const prepare = () =>
    new Promise<() => void>((resolve) => {
      ready = resolve;
    });
  const view = render(<App scope="old" prepare={prepare} />);
  await screen.findByRole('dialog');
  await user.click(screen.getByRole('button', { name: 'Open join form' }));
  view.rerender(<App scope="new" prepare={prepare} />);
  await act(async () => {
    ready?.(commit);
  });
  expect(await screen.findByRole('alert')).toHaveTextContent('Could not use the link');
  expect(commit).not.toHaveBeenCalled();
});
