import { describe, expect, it } from 'vitest';
import { type DownloadState, mergeManifests, type SystemEntry, type UserEntry } from '../manifest';

function makeSystemEntry(id: string, overrides: Partial<SystemEntry> = {}): SystemEntry {
  return {
    id,
    hf_repo: `org/${id}`,
    hf_filename: `${id}.gguf`,
    arch: 'llama',
    quant: 'Q4_K_M',
    size_gb: 4.2,
    license: 'apache-2.0',
    display_name: id,
    ...overrides,
  };
}

function makeUserEntry(id: string, overrides: Partial<UserEntry> = {}): UserEntry {
  return {
    ...makeSystemEntry(id),
    added_at: '2026-05-01T00:00:00Z',
    source: 'hf-search',
    ...overrides,
  };
}

describe('mergeManifests', () => {
  it('returns system + user entries marked with origin', () => {
    const system = [makeSystemEntry('sys-a'), makeSystemEntry('sys-b')];
    const user = [makeUserEntry('user-a')];

    const merged = mergeManifests(system, user, new Set(), new Map());

    expect(merged).toHaveLength(3);
    expect(merged[0]).toMatchObject({ id: 'sys-a', source: 'system' });
    expect(merged[1]).toMatchObject({ id: 'sys-b', source: 'system' });
    expect(merged[2]).toMatchObject({ id: 'user-a', source: 'user' });
  });

  it('marks installed flag from installedIds set', () => {
    const system = [makeSystemEntry('sys-a'), makeSystemEntry('sys-b')];
    const user = [makeUserEntry('user-a')];
    const installed = new Set(['sys-b', 'user-a']);

    const merged = mergeManifests(system, user, installed, new Map());

    expect(merged.find((m) => m.id === 'sys-a')?.installed).toBe(false);
    expect(merged.find((m) => m.id === 'sys-b')?.installed).toBe(true);
    expect(merged.find((m) => m.id === 'user-a')?.installed).toBe(true);
  });

  it('attaches download state and progress', () => {
    const system = [makeSystemEntry('sys-a')];
    const user: UserEntry[] = [];
    const states = new Map<string, DownloadState>([
      ['sys-a', { state: 'downloading', progress: 0.42 }],
    ]);

    const merged = mergeManifests(system, user, new Set(), states);
    const entry = merged[0];
    if (!entry) throw new Error('expected first entry');

    expect(entry.downloadState).toBe('downloading');
    expect(entry.downloadProgress).toBe(0.42);
    expect(entry.errorMessage).toBeUndefined();
  });

  it('returns empty array when both sources empty', () => {
    const merged = mergeManifests([], [], new Set(), new Map());
    expect(merged).toEqual([]);
  });
});
