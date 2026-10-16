import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import { RoomInvitationShare } from './RoomInvitationShare';

beforeEach(async () => {
  vi.clearAllMocks();
  await i18n.changeLanguage('en');
});
const props = {
  address: '127.0.0.1:4123',
  roomId: '00000000-0000-4000-8000-000000000001',
  invitation: 'a'.repeat(64),
  busy: false,
};
it('requires a reachable advertised address when the host listens on all interfaces', async () => {
  const user = userEvent.setup();
  render(<RoomInvitationShare {...props} address="0.0.0.0:4123" />);
  expect(screen.getByRole('button', { name: 'Create invitation link' })).toBeDisabled();
  await user.type(screen.getByLabelText('Address to share with players'), '192.168.1.20:4123');
  await user.click(screen.getByRole('button', { name: 'Create invitation link' }));
  expect((await screen.findByLabelText<HTMLInputElement>('Invitation link')).value).toContain(
    'dmai://join/v1?',
  );
});
it('discards a rotated link and does not let a stale clipboard result label the next one copied', async () => {
  const user = userEvent.setup();
  const view = render(<RoomInvitationShare {...props} />);
  await user.click(screen.getByRole('button', { name: 'Create invitation link' }));
  await screen.findByLabelText('Invitation link');
  let done: (() => void) | undefined;
  vi.spyOn(navigator.clipboard, 'writeText').mockImplementation(
    () =>
      new Promise<void>((resolve) => {
        done = resolve;
      }),
  );
  await user.click(screen.getByRole('button', { name: 'Copy link' }));
  view.rerender(<RoomInvitationShare {...props} invitation={'b'.repeat(64)} />);
  expect(screen.queryByLabelText('Invitation link')).not.toBeInTheDocument();
  await act(async () => {
    done?.();
  });
  await user.click(screen.getByRole('button', { name: 'Create invitation link' }));
  expect(await screen.findByRole('button', { name: 'Copy link' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Link copied' })).not.toBeInTheDocument();
});
