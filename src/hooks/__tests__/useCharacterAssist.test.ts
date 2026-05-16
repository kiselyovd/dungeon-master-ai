import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import * as assistApi from '../../api/characterAssist';
import * as useStoreModule from '../../state/useStore';
import { useCharacterAssist } from '../useCharacterAssist';

afterEach(() => {
  vi.restoreAllMocks();
  useStoreModule.useStore.getState().charCreation.resetDraft();
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

  it('generateField for personality_flag writes flag via slice action on done', async () => {
    const spy = vi.spyOn(assistApi, 'streamCharacterField').mockImplementation(async (args) => {
      args.onToken('I keep a coin ');
      args.onToken('from my mentor.');
      args.onDone();
    });
    const { result } = renderHook(() => useCharacterAssist());
    await act(async () => {
      await result.current.generateField('personality_flag', {
        slotId: 'bg-trait',
        source: 'background',
        sourceLabel: 'Acolyte',
        pool: ['I see omens in every event.'],
      });
    });
    expect(spy).toHaveBeenCalled();
    const callArgs = spy.mock.calls[0]?.[0];
    expect(callArgs?.field).toBe('personality_flag');
    expect(callArgs?.flagContext?.slotId).toBe('bg-trait');
    const flags = useStoreModule.useStore.getState().charCreation.personalityFlags;
    expect(flags).toEqual([
      { slotId: 'bg-trait', source: 'background', flag: 'I keep a coin from my mentor.' },
    ]);
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
