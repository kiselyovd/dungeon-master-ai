import { describe, expect, it } from 'vitest';
import {
  commandPlatform,
  filterCommands,
  matchesShortcut,
  PALETTE_SHORTCUT,
  shortcutLabel,
} from './model';

describe('command shortcuts', () => {
  it.each(['Win32', 'Linux x86_64'])('uses Ctrl on %s', (platform) => {
    expect(commandPlatform(platform)).toBe('other');
    expect(
      matchesShortcut(
        new KeyboardEvent('keydown', { key: 'K', ctrlKey: true }),
        PALETTE_SHORTCUT,
        'other',
      ),
    ).toBe(true);
    expect(
      matchesShortcut(
        new KeyboardEvent('keydown', { key: 'k', metaKey: true }),
        PALETTE_SHORTCUT,
        'other',
      ),
    ).toBe(false);
    expect(shortcutLabel(PALETTE_SHORTCUT, 'other')).toBe('Ctrl+K');
  });
  it('uses Meta on macOS without also accepting Ctrl', () => {
    expect(commandPlatform('MacIntel')).toBe('mac');
    expect(
      matchesShortcut(
        new KeyboardEvent('keydown', { key: 'k', metaKey: true }),
        PALETTE_SHORTCUT,
        'mac',
      ),
    ).toBe(true);
    expect(
      matchesShortcut(
        new KeyboardEvent('keydown', { key: 'k', ctrlKey: true }),
        PALETTE_SHORTCUT,
        'mac',
      ),
    ).toBe(false);
    expect(shortcutLabel({ key: 's', primary: true, shift: true }, 'mac')).toBe('⌘+Shift+S');
  });
  it.each([
    { repeat: true },
    { isComposing: true },
    { keyCode: 229 },
    { altKey: true },
    { shiftKey: true },
    { metaKey: true },
  ])('ignores composition, repeats and extra modifiers %j', (extra) => {
    expect(
      matchesShortcut(
        new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, ...extra }),
        PALETTE_SHORTCUT,
        'other',
      ),
    ).toBe(false);
  });
  it('does not interpret normal input or newlines as commands', () => {
    expect(
      matchesShortcut(
        new KeyboardEvent('keydown', { key: 'Enter', shiftKey: true }),
        PALETTE_SHORTCUT,
        'other',
      ),
    ).toBe(false);
    expect(
      matchesShortcut(new KeyboardEvent('keydown', { key: 'k' }), PALETTE_SHORTCUT, 'other'),
    ).toBe(false);
  });
  it('filters localized labels and keeps diagnostics advanced even during search', () => {
    const commands = [
      { id: 'hero', label: 'Открыть лист героя', enabled: true, run() {} },
      { id: 'debug', label: 'Диагностика', advanced: true, enabled: true, run() {} },
    ];
    expect(filterCommands(commands, 'героя открыть', false).map((item) => item.id)).toEqual([
      'hero',
    ]);
    expect(filterCommands(commands, 'Диагностика', false)).toEqual([]);
    expect(filterCommands(commands, 'Диагностика', true)).toHaveLength(1);
  });
});
