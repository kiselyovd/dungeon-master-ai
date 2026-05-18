/**
 * Local LLM manifest merge logic.
 *
 * Combines the system-shipped model catalog with user-added entries (from HF
 * search) into a single list, enriched with install status and live download
 * state. Consumed by ModelSelector + ActiveModelPicker + ManageDownloads.
 */

export interface SystemEntry {
  id: string;
  hf_repo: string;
  hf_filename: string;
  arch: string;
  quant: string;
  size_gb: number;
  license: string;
  display_name: string;
}

export interface UserEntry extends SystemEntry {
  added_at: string;
  source: 'hf-search';
}

export interface DownloadState {
  state: 'idle' | 'queued' | 'downloading' | 'verifying' | 'error';
  progress?: number;
  errorMessage?: string;
}

export interface MergedEntry extends SystemEntry {
  source: 'system' | 'user';
  installed: boolean;
  downloadState?: DownloadState['state'];
  downloadProgress?: number;
  errorMessage?: string;
}

function buildEntry<TSource extends 'system' | 'user'>(
  base: SystemEntry,
  source: TSource,
  installed: boolean,
  ds: DownloadState | undefined,
): MergedEntry {
  const merged: MergedEntry = {
    ...base,
    source,
    installed,
  };
  if (ds?.state !== undefined) merged.downloadState = ds.state;
  if (ds?.progress !== undefined) merged.downloadProgress = ds.progress;
  if (ds?.errorMessage !== undefined) merged.errorMessage = ds.errorMessage;
  return merged;
}

export function mergeManifests(
  system: SystemEntry[],
  user: UserEntry[],
  installedIds: Set<string>,
  downloadStates: Map<string, DownloadState>,
): MergedEntry[] {
  const out: MergedEntry[] = [];
  for (const s of system) {
    out.push(buildEntry(s, 'system', installedIds.has(s.id), downloadStates.get(s.id)));
  }
  for (const u of user) {
    out.push(buildEntry(u, 'user', installedIds.has(u.id), downloadStates.get(u.id)));
  }
  return out;
}
