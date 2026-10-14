import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { resetDesktopGeometry } from '../api/desktop';
import type { AppCommand } from '../features/commands/model';
import { useStore } from '../state/useStore';

export interface CommandContext {
  table: boolean;
  guest: boolean;
  room: boolean;
  busy: boolean;
  recovery: boolean;
  ownTurn: boolean;
}
export type CommandHandlers = Partial<Record<string, () => unknown | Promise<unknown>>>;
export function registerCommands(
  context: CommandContext,
  label: (id: string) => string,
  handlers: CommandHandlers,
): AppCommand[] {
  const entries: {
    id: string;
    enabled: boolean;
    key?: string;
    shift?: boolean;
    advanced?: boolean;
    allowEditing?: boolean;
  }[] = [
    { id: 'hero', enabled: context.table, key: 'h', shift: true },
    { id: 'journal', enabled: context.table, key: 'j', shift: true },
    { id: 'npcs', enabled: context.table, key: 'n', shift: true },
    { id: 'saves', enabled: context.table && !context.room, key: 's', shift: true },
    {
      id: 'quick_save',
      enabled: context.table && !context.room && !context.busy && !context.recovery,
      key: 's',
      allowEditing: true,
    },
    {
      id: 'recovery',
      enabled: context.table && context.recovery && !context.busy,
      key: 'r',
      shift: true,
    },
    { id: 'room', enabled: true },
    { id: 'settings', enabled: !context.guest, key: ',' },
    { id: 'focus_chat', enabled: context.table, key: 'c', shift: true },
    { id: 'focus_map', enabled: context.table },
    { id: 'reset_layout', enabled: context.table },
    {
      id: 'end_turn',
      enabled: context.table && context.ownTurn && !context.busy && !context.recovery,
    },
    { id: 'fullscreen', enabled: true },
    { id: 'local_runtime', enabled: !context.guest, key: 'm', shift: true, advanced: true },
    {
      id: 'inspector',
      enabled: context.table && !context.guest,
      key: 'i',
      shift: true,
      advanced: true,
    },
  ];
  return entries.map((entry) => ({
    id: entry.id,
    label: label(entry.id),
    enabled: entry.enabled && Boolean(handlers[entry.id]),
    unavailable: label(
      context.guest && ['settings', 'quick_save', 'saves'].includes(entry.id)
        ? 'host_only'
        : entry.id === 'end_turn'
          ? 'own_turn_only'
          : 'unavailable',
    ),
    ...(entry.advanced ? { advanced: true } : {}),
    ...(entry.id === 'fullscreen'
      ? { shortcut: { key: 'F11', allowEditing: true } }
      : entry.key
        ? {
            shortcut: {
              key: entry.key,
              primary: true,
              shift: Boolean(entry.shift),
              allowEditing: Boolean(entry.allowEditing),
            },
          }
        : {}),
    run: handlers[entry.id] ?? (() => {}),
  }));
}
export async function toggleFullscreen(): Promise<void> {
  if (isTauri()) {
    const win = getCurrentWindow();
    await win.setFullscreen(!(await win.isFullscreen()));
  } else if (document.fullscreenElement) await document.exitFullscreen();
  else await document.documentElement.requestFullscreen();
}
export async function resetLayout(): Promise<void> {
  useStore.getState().settings.resetWorkspace();
  await resetDesktopGeometry();
}
export function focusTable(target: 'chat' | 'map' | 'room'): void {
  if (target === 'room') {
    document.querySelector<HTMLButtonElement>('[data-command-target="room"]')?.click();
    return;
  }
  const workspace = useStore.getState().settings;
  if (target === 'chat' && workspace.workspace.viewMode === 'scene')
    workspace.setWorkspace({ viewMode: 'split' });
  if (target === 'map' && workspace.workspace.viewMode === 'chat')
    workspace.setWorkspace({ viewMode: 'split' });
  window.requestAnimationFrame(() =>
    document.querySelector<HTMLElement>(`[data-command-target="${target}"]`)?.focus(),
  );
}
