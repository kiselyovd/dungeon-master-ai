import { describe, expect, it, vi } from 'vitest';
import { type CommandContext, registerCommands } from '../commandRegistrations';

const base: CommandContext = {
  table: true,
  guest: false,
  room: false,
  busy: false,
  recovery: false,
  ownTurn: true,
};
const ids = [
  'hero',
  'journal',
  'npcs',
  'saves',
  'quick_save',
  'recovery',
  'room',
  'settings',
  'focus_chat',
  'focus_map',
  'reset_layout',
  'end_turn',
  'fullscreen',
  'local_runtime',
  'inspector',
];
const handlers = Object.fromEntries(ids.map((id) => [id, vi.fn()]));
function enabled(context: CommandContext, id: string) {
  return registerCommands(context, (key) => key, handlers).find((command) => command.id === id)
    ?.enabled;
}
describe('app role-aware command composition', () => {
  it('refuses guest settings/saves and all room local-save operations', () => {
    const guest = { ...base, guest: true, room: true };
    for (const id of ['saves', 'quick_save', 'settings', 'local_runtime', 'inspector'])
      expect(enabled(guest, id)).toBe(false);
    expect(enabled({ ...base, room: true }, 'quick_save')).toBe(false);
    expect(enabled(guest, 'journal')).toBe(true);
  });
  it.each([
    { ownTurn: false },
    { busy: true },
    { recovery: true },
    { table: false },
  ])('refuses end-turn with %j', (patch) =>
    expect(enabled({ ...base, ...patch }, 'end_turn')).toBe(false));
  it('guards save and recovery independently and requires a real callback', () => {
    expect(enabled({ ...base, busy: true }, 'quick_save')).toBe(false);
    expect(enabled({ ...base, recovery: true }, 'quick_save')).toBe(false);
    expect(enabled(base, 'recovery')).toBe(false);
    expect(enabled({ ...base, recovery: true }, 'recovery')).toBe(true);
    expect(registerCommands(base, (id) => id, {}).every((command) => !command.enabled)).toBe(true);
  });
});
