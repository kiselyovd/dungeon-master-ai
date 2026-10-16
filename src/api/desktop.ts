import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

/** Native filesystem and window policy stays behind the main-window command. */
export async function resetDesktopGeometry(): Promise<void> {
  if (isTauri()) await invoke('desktop_reset_geometry');
}

export async function listenForDesktopClose(request: () => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  const window = getCurrentWindow();
  if (window.label !== 'main') return () => {};
  return window.onCloseRequested((event) => {
    event.preventDefault();
    request();
  });
}
export async function confirmDesktopExit(): Promise<void> {
  if (!isTauri()) throw Error('desktop_unavailable');
  await invoke('desktop_confirm_exit');
}
