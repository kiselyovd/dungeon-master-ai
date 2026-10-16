import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

export type AuxiliaryKind = 'hero' | 'journal';
export const auxiliaryWindowsAvailable = isTauri;
export async function publishAuxiliaryProjection(frame: unknown): Promise<void> {
  if (isTauri()) await invoke('desktop_aux_publish', { frame });
}
export async function heartbeatAuxiliaryProjection(sequence: number): Promise<void> {
  if (isTauri()) await invoke('desktop_aux_heartbeat', { sequence });
}
export async function openAuxiliaryWindow(kind: AuxiliaryKind): Promise<void> {
  if (isTauri()) await invoke('desktop_aux_open', { kind });
}
export async function readAuxiliaryProjection(afterSequence: number | null): Promise<unknown> {
  return isTauri() ? invoke('desktop_aux_read', { afterSequence }) : null;
}
export function auxiliaryKind(): AuxiliaryKind | null {
  const label = isTauri()
    ? getCurrentWindow().label
    : new URLSearchParams(location.search).get('view');
  return label === 'hero' || label === 'journal' ? label : null;
}
export async function closeAuxiliaryWindow(): Promise<void> {
  if (isTauri()) await invoke('desktop_aux_close');
}
export async function focusMainWindow(): Promise<void> {
  if (isTauri()) await invoke('desktop_aux_focus_main');
}
