import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import * as assistApi from '../../api/characterAssist';
import { useCharacterAssist } from '../useCharacterAssist';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('useCharacterAssist', () => {
  it('generateField streams tokens', async () => {
    const spy = vi.spyOn(assistApi, 'streamCharacterField').mockImplementation(async (args) => {
      args.onToken('Alpha');
      args.onToken(' Beta');
      args.onDone();
    });
    const { result } = renderHook(() => useCharacterAssist());
    await act(async () => {
      await result.current.generateField('name');
    });
    expect(spy).toHaveBeenCalled();
    expect(spy.mock.calls[0]?.[0].field).toBe('name');
  });

  it('surpriseMe applies patches', async () => {
    const spy = vi.spyOn(assistApi, 'streamFullCharacter').mockImplementation(async (args) => {
      args.onPatch({ classId: 'fighter' });
      args.onPatch({ raceId: 'human' });
      args.onDone();
    });
    const { result } = renderHook(() => useCharacterAssist());
    await act(async () => {
      await result.current.surpriseMe();
    });
    expect(spy).toHaveBeenCalled();
  });

  it('runTestChat returns concatenated streamed reply', async () => {
    vi.spyOn(assistApi, 'streamTestChat').mockImplementation(async (args) => {
      args.onToken('A grizzled barkeep eyes you. ');
      args.onToken('What brings you?');
      args.onDone();
    });
    const { result } = renderHook(() => useCharacterAssist());
    let final = '';
    await act(async () => {
      final = await result.current.runTestChat('Hi', []);
    });
    expect(final).toBe('A grizzled barkeep eyes you. What brings you?');
  });
});
