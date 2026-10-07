import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import '../../i18n';
import wire from '../../api/__fixtures__/roomUpdate.json';
import { RoomExperience } from '../RoomExperience';

const mocks = vi.hoisted(() => ({ host: vi.fn(), current: vi.fn() }));
vi.mock('../../api/rooms', () => ({
  hostRoom: mocks.host,
  joinRoom: vi.fn(),
  stopHostRoom: vi.fn(async () => {}),
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
