import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import '../../i18n';
import wire from '../../api/__fixtures__/roomUpdate.json';
import { RoomExperience } from '../RoomExperience';

const mocks = vi.hoisted(() => ({
  host: vi.fn(),
  recoverHost: vi.fn(),
  current: vi.fn(),
  saved: vi.fn(),
  postpone: vi.fn(),
  resume: vi.fn(),
  leave: vi.fn(),
}));
vi.mock('../../api/rooms', () => ({
  recoverHostRoom: mocks.recoverHost,
  recoverGuestRoom: vi.fn(async () => null),
  savedGuestRooms: mocks.saved,
  postponeGuestRoom: mocks.postpone,
  resumeGuestRoom: mocks.resume,
  hostRoom: mocks.host,
  joinRoom: vi.fn(),
  stopHostRoom: vi.fn(async () => {}),
  leaveGuestRoom: mocks.leave,
}));
vi.mock('../../api/campaigns', () => ({ fetchCampaignState: mocks.current }));
vi.mock('../roomPorts', () => ({
  roomPorts: {
    load: async () => wire,
    pending: async () => null,
    act: async () => {},
    stream: () => new Promise<void>(() => {}),
  },
}));
const campaign = {
  id: wire.state.campaign.id,
  schema_version: 1 as const,
  ruleset_id: 'srd-5.2.1' as const,
  title: 'Host campaign',
  premise: 'River',
  last_session_id: wire.state.room.session_id,
  revision: 4,
  updated_at: '2026-10-07T12:00:00Z',
};
beforeEach(() => {
  mocks.host.mockReset();
  mocks.recoverHost.mockReset();
  mocks.saved.mockResolvedValue([]);
  mocks.postpone.mockResolvedValue(undefined);
  mocks.leave.mockReset();
  mocks.current.mockReset();
  mocks.current.mockResolvedValue({
    session_id: campaign.last_session_id,
    campaign: { revision: 4 },
  });
});
it('uses a fresh creation UUID and revision after a definitive bind failure', async () => {
  mocks.host.mockRejectedValueOnce({ code: 'native_room_bind_failed' }).mockResolvedValue({
    kind: 'host',
    room: wire.state.room,
    member_id: wire.state.room.members[0]?.member_id,
    address: '127.0.0.1:4123',
    invitation: 'fixture',
  });
  const user = userEvent.setup();
  render(<RoomExperience campaigns={[campaign]} onBack={() => {}} />);
  await user.click(screen.getByRole('button', { name: 'Create a room' }));
  await screen.findByRole('alert');
  mocks.current.mockResolvedValue({
    session_id: campaign.last_session_id,
    campaign: { revision: 5 },
  });
  await user.click(screen.getByRole('button', { name: 'Create a room' }));
  await screen.findByRole('heading', { name: 'Dungeon Master · AI' });
  expect(mocks.host.mock.calls[0]?.[0].request_id).not.toBe(
    mocks.host.mock.calls[1]?.[0].request_id,
  );
  expect(mocks.host.mock.calls[1]?.[0].expected_revision).toBe(5);
});
it('retains creation UUID after an unknown delivery failure', async () => {
  mocks.host.mockRejectedValueOnce({ code: 'native_host_request_failed' }).mockResolvedValue({
    kind: 'host',
    room: wire.state.room,
    member_id: wire.state.room.members[0]?.member_id,
    address: '127.0.0.1:4123',
    invitation: 'fixture',
  });
  const user = userEvent.setup();
  render(<RoomExperience campaigns={[campaign]} onBack={() => {}} />);
  await user.click(screen.getByRole('button', { name: 'Create a room' }));
  await screen.findByRole('alert');
  await user.click(screen.getByRole('button', { name: 'Create a room' }));
  await screen.findByRole('heading', { name: 'Dungeon Master · AI' });
  expect(mocks.host.mock.calls[0]?.[0]).toEqual(mocks.host.mock.calls[1]?.[0]);
});

it('explicitly postpones a pending join and restores its preserved archived handle', async () => {
  const choice = {
    room_id: wire.state.room.room_id,
    join_id: '77777777-7777-4777-8777-777777777777',
    address: '127.0.0.1:4123',
    display_name: 'Saved player',
  };
  mocks.saved.mockResolvedValueOnce([]).mockResolvedValue([choice]);
  mocks.resume.mockResolvedValue({
    kind: 'guest',
    room: wire.state.room,
    member_id: wire.state.room.members.find((member) => member.role === 'player')?.member_id,
    address: choice.address,
    join_id: choice.join_id,
  });
  const user = userEvent.setup();
  render(<RoomExperience campaigns={[]} onBack={() => {}} />);
  await user.click(screen.getByRole('button', { name: 'Postpone the pending connection' }));
  expect(mocks.postpone).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Keep it for later and try another room' }));
  const saved = await screen.findByRole('button', { name: 'Saved player · 127.0.0.1:4123' });
  await user.click(saved);
  await screen.findByRole('heading', { name: 'Dungeon Master · AI' });
  expect(mocks.resume).toHaveBeenCalledWith(choice);
});

it('recovers an existing host selection without creating a replacement room', async () => {
  mocks.recoverHost.mockRejectedValueOnce(new Error('port busy')).mockResolvedValueOnce({
    kind: 'host',
    room: wire.state.room,
    member_id: wire.state.room.members.find((member) => member.role === 'host')?.member_id,
    address: '127.0.0.1:4123',
    invitation: 'a'.repeat(64),
  });
  const user = userEvent.setup();
  render(<RoomExperience campaigns={[]} onBack={() => {}} />);
  await user.click(screen.getByRole('button', { name: 'Recover hosted room' }));
  await screen.findByRole('alert');
  await user.click(screen.getByRole('button', { name: 'Recover hosted room' }));
  await screen.findByRole('heading', { name: 'Dungeon Master · AI' });
  expect(mocks.recoverHost).toHaveBeenCalledTimes(2);
  expect(mocks.host).not.toHaveBeenCalled();
  expect(mocks.current).not.toHaveBeenCalled();
  mocks.recoverHost.mockResolvedValueOnce({
    kind: 'host',
    room: wire.state.room,
    member_id: wire.state.room.members.find((member) => member.role === 'host')?.member_id,
    address: '127.0.0.1:4123',
    invitation: 'b'.repeat(64),
  });
  await user.click(screen.getByRole('button', { name: 'Room' }));
  await user.click(screen.getByRole('button', { name: 'Create invitation link' }));
  const originalLink = await screen.findByLabelText<HTMLInputElement>('Invitation link');
  expect(new URL(originalLink.value).searchParams.get('invitation')).toBe('a'.repeat(64));
  await user.click(screen.getByRole('button', { name: 'Refresh invitation' }));
  expect(screen.queryByLabelText('Invitation link')).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Create invitation link' }));
  const renewedLink = await screen.findByLabelText<HTMLInputElement>('Invitation link');
  const parsed = new URL(renewedLink.value);
  expect(parsed.searchParams.get('invitation')).toBe('b'.repeat(64));
  expect(parsed.searchParams.get('room')).toBe(wire.state.room.room_id);
  expect(mocks.recoverHost).toHaveBeenCalledTimes(3);
  expect(mocks.host).not.toHaveBeenCalled();
});

it('waits for confirmed guest departure before leaving the connected screen', async () => {
  const choice = {
    room_id: wire.state.room.room_id,
    join_id: '77777777-7777-4777-8777-777777777777',
    address: '127.0.0.1:4123',
    display_name: 'Saved player',
  };
  mocks.saved.mockResolvedValue([choice]);
  mocks.resume.mockResolvedValue({
    kind: 'guest',
    room: wire.state.room,
    member_id: wire.state.room.members.find((m) => m.role === 'player')?.member_id,
    address: choice.address,
    join_id: choice.join_id,
  });
  let confirm: () => void = () => {};
  mocks.leave.mockImplementation(
    () =>
      new Promise<void>((resolve) => {
        confirm = resolve;
      }),
  );
  const back = vi.fn();
  const user = userEvent.setup();
  render(<RoomExperience campaigns={[]} onBack={back} />);
  await user.click(await screen.findByRole('button', { name: 'Saved player · 127.0.0.1:4123' }));
  await screen.findByRole('heading', { name: 'Dungeon Master · AI' });
  if (!screen.queryByRole('dialog', { name: 'Room' }))
    await user.click(screen.getByRole('button', { name: 'Room' }));
  await user.click(screen.getByRole('button', { name: 'Leave room' }));
  expect(back).not.toHaveBeenCalled();
  expect(mocks.leave).toHaveBeenCalledWith(wire.state.room.room_id, 1);
  confirm();
  await vi.waitFor(() => expect(back).toHaveBeenCalledTimes(1));
});

it('retries only local navigation after a confirmed guest departure', async () => {
  const choice = {
    room_id: wire.state.room.room_id,
    join_id: '77777777-7777-4777-8777-777777777777',
    address: '127.0.0.1:4123',
    display_name: 'Saved player',
  };
  mocks.saved.mockResolvedValue([choice]);
  mocks.resume.mockResolvedValue({
    kind: 'guest',
    room: wire.state.room,
    member_id: wire.state.room.members.find((m) => m.role === 'player')?.member_id,
    address: choice.address,
    join_id: choice.join_id,
  });
  mocks.leave.mockResolvedValueOnce(undefined).mockRejectedValue(new Error('already left'));
  const back = vi
    .fn()
    .mockRejectedValueOnce(new Error('local activation failed'))
    .mockResolvedValue(undefined);
  const user = userEvent.setup();
  render(<RoomExperience campaigns={[]} onBack={back} />);
  await user.click(await screen.findByRole('button', { name: 'Saved player · 127.0.0.1:4123' }));
  await screen.findByRole('heading', { name: 'Dungeon Master · AI' });
  if (!screen.queryByRole('dialog', { name: 'Room' }))
    await user.click(screen.getByRole('button', { name: 'Room' }));
  await user.click(screen.getByRole('button', { name: 'Leave room' }));
  await screen.findByRole('alert');
  if (!screen.queryByRole('dialog', { name: 'Room' }))
    await user.click(screen.getByRole('button', { name: 'Room' }));
  await user.click(screen.getByRole('button', { name: 'Leave room' }));
  await vi.waitFor(() => expect(back).toHaveBeenCalledTimes(2));
  expect(mocks.leave).toHaveBeenCalledTimes(1);
});
