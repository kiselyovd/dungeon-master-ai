import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import { RoomHome } from './RoomHome';

const preview = {
  address: '127.0.0.1:4123',
  room_id: '00000000-0000-4000-8000-000000000001',
  invitation: 'a'.repeat(64),
};
const callbacks = { onHost: vi.fn(), onJoin: vi.fn(), onBack: vi.fn() };
beforeEach(async () => {
  vi.clearAllMocks();
  await i18n.changeLanguage('en');
});
it('checks a pasted link without joining and clears the raw URI before confirmation', async () => {
  const user = userEvent.setup();
  render(<RoomHome campaigns={[]} busy={false} error={false} {...callbacks} />);
  expect(screen.getByRole('button', { name: 'Join a room' })).toBeDisabled();
  await user.type(
    screen.getByLabelText('Invitation link'),
    `dmai://join/v1?address=${preview.address}&room=${preview.room_id}&invitation=${preview.invitation}`,
  );
  await user.click(screen.getByRole('button', { name: 'Check link' }));
  expect(await screen.findByRole('region', { name: 'Room invitation' })).toHaveTextContent(
    preview.address,
  );
  expect(screen.getByLabelText('Invitation link')).toHaveValue('');
  expect(callbacks.onJoin).not.toHaveBeenCalled();
  await user.type(screen.getByLabelText('Your player name'), 'Tara');
  await user.click(screen.getByRole('button', { name: 'Join a room' }));
  expect(callbacks.onJoin).toHaveBeenCalledOnce();
  expect(callbacks.onJoin).toHaveBeenCalledWith(
    preview.address,
    preview.room_id,
    'Tara',
    preview.invitation,
  );
});
it('rejects a malformed link and retains the player name for repair', async () => {
  const user = userEvent.setup();
  render(<RoomHome campaigns={[]} busy={false} error={false} {...callbacks} />);
  await user.type(screen.getByLabelText('Your player name'), 'Tara');
  await user.type(screen.getByLabelText('Invitation link'), 'https://invalid.example');
  await user.click(screen.getByRole('button', { name: 'Check link' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Could not use the link');
  expect(callbacks.onJoin).not.toHaveBeenCalled();
  expect(screen.getByLabelText('Your player name')).toHaveValue('Tara');
});
it('receives a new native preview without submitting or losing the typed player name', async () => {
  const view = render(
    <RoomHome
      initialInvitation={preview}
      campaigns={[]}
      busy={false}
      error={false}
      {...callbacks}
    />,
  );
  fireEvent.change(screen.getByLabelText('Your player name'), { target: { value: 'Tara' } });
  view.rerender(
    <RoomHome
      initialInvitation={{ ...preview, address: '192.168.1.20:4123' }}
      campaigns={[]}
      busy={false}
      error={false}
      {...callbacks}
    />,
  );
  await waitFor(() =>
    expect(screen.getByRole('region', { name: 'Room invitation' })).toHaveTextContent(
      '192.168.1.20:4123',
    ),
  );
  expect(callbacks.onJoin).not.toHaveBeenCalled();
  expect(screen.getByLabelText('Your player name')).toHaveValue('Tara');
});
