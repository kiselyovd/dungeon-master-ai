import { describe, expect, it } from 'vitest';
import { createInvitation, parseBrowserInvitation } from './invitations';

const preview = {
  address: '127.0.0.1:4123',
  room_id: '00000000-0000-4000-8000-000000000001',
  invitation: 'a'.repeat(64),
};
const link = `dmai://join/v1?address=${preview.address}&room=${preview.room_id}&invitation=${preview.invitation}`;
describe('invitation grammar', () => {
  it('roundtrips only the existing join address, room and expiring invitation', async () => {
    expect(parseBrowserInvitation(await createInvitation(preview))).toEqual(preview);
    expect(parseBrowserInvitation(link)).toEqual(preview);
    expect(
      parseBrowserInvitation(link.replace('127.0.0.1:4123', '%5B%3A%3A1%5D%3A4123')).address,
    ).toBe('[::1]:4123');
  });
  it.each([
    link.replace('dmai:', 'https:'),
    link.replace('/v1', '/v2'),
    link.replace('/v1', '/x/../v1'),
    `${link}#`,
    `${link}&capability=private`,
    `${link}&room=${preview.room_id}`,
    link.replace('join/', 'person@join/'),
    link.replace('join/', 'join:80/'),
    link.replace(preview.address, '0.0.0.0:4123'),
    link.replace(preview.address, '224.1.2.3:4123'),
    link.replace(preview.address, '127.0.0.1:0'),
    link.replace(preview.address, '127.0.0.1:65536'),
    link.replace(preview.address, 'localhost:4123'),
    link.replace(preview.address, '001.2.3.4:4123'),
    link.replace('invitation=', 'resume_proof='),
    link.replace(preview.invitation, 'short'),
    `${link}\n`,
    'a'.repeat(1025),
  ])('rejects malformed, duplicated and extra fields without echoing the link (%#)', (raw) => {
    expect(() => parseBrowserInvitation(raw)).toThrow('native_invitation_invalid');
  });
});
