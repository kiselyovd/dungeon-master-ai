import { invoke, isTauri } from '@tauri-apps/api/core';

/** Native filesystem and window policy stays behind the main-window command. */
export async function resetDesktopGeometry(): Promise<void> {
  if (isTauri()) await invoke('desktop_reset_geometry');
}
