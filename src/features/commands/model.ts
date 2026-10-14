export type CommandPlatform = 'mac' | 'other';
export interface Shortcut {
  key: string;
  primary?: boolean;
  shift?: boolean;
  allowEditing?: boolean;
}
export interface AppCommand {
  id: string;
  label: string;
  enabled: boolean;
  unavailable?: string;
  advanced?: boolean;
  shortcut?: Shortcut;
  run(): unknown | Promise<unknown>;
}
export const PALETTE_SHORTCUT: Shortcut = { key: 'k', primary: true, allowEditing: true };
export function commandPlatform(platform: string): CommandPlatform {
  return /mac|iphone|ipad/i.test(platform) ? 'mac' : 'other';
}
export function shortcutLabel(shortcut: Shortcut, platform: CommandPlatform): string {
  return [
    shortcut.primary ? (platform === 'mac' ? '⌘' : 'Ctrl') : '',
    shortcut.shift ? 'Shift' : '',
    shortcut.key.length === 1 ? shortcut.key.toUpperCase() : shortcut.key,
  ]
    .filter(Boolean)
    .join('+');
}
export function matchesShortcut(
  event: KeyboardEvent,
  shortcut: Shortcut,
  platform: CommandPlatform,
  allowRepeat = false,
): boolean {
  if (
    event.defaultPrevented ||
    event.isComposing ||
    event.keyCode === 229 ||
    (event.repeat && !allowRepeat) ||
    event.altKey
  )
    return false;
  const primary = platform === 'mac' ? event.metaKey : event.ctrlKey;
  const foreign = platform === 'mac' ? event.ctrlKey : event.metaKey;
  if (
    foreign ||
    primary !== Boolean(shortcut.primary) ||
    event.shiftKey !== Boolean(shortcut.shift)
  )
    return false;
  if (
    !shortcut.allowEditing &&
    event.target instanceof Element &&
    event.target.closest('input,textarea,select,[contenteditable="true"],[role="textbox"]')
  )
    return false;
  return event.key.toLowerCase() === shortcut.key.toLowerCase();
}
export function filterCommands(
  commands: readonly AppCommand[],
  query: string,
  advanced: boolean,
): AppCommand[] {
  const words = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  return commands.filter(
    (command) =>
      (!command.advanced || advanced) &&
      words.every((word) => command.label.toLocaleLowerCase().includes(word)),
  );
}
