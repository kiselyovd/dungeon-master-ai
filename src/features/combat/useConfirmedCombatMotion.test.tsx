import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { CombatMotionFrame } from './model/confirmedMotion';
import { useConfirmedCombatMotion } from './useConfirmedCombatMotion';

const frame: CombatMotionFrame = {
  scopeKey: 'campaign:timeline-1:encounter',
  revision: 4,
  round: 1,
  active: true,
  ownActorId: 'hero',
  currentTurnId: 'hero',
  tokens: [
    {
      id: 'hero',
      name: 'Hero',
      characterId: 'sheet',
      hp: 12,
      maxHp: 12,
      ac: 18,
      x: 1,
      y: 3,
      conditions: [],
    },
  ],
};
const damaged = (revision: number, hp: number): CombatMotionFrame => ({
  ...frame,
  revision,
  tokens: frame.tokens.map((token) => ({ ...token, hp })),
});
let preferenceChanged: (() => void) | undefined;
let reduced = false;
beforeEach(() => {
  vi.useFakeTimers();
  reduced = false;
  preferenceChanged = undefined;
  vi.spyOn(document, 'hasFocus').mockReturnValue(true);
  vi.stubGlobal('matchMedia', () => ({
    get matches() {
      return reduced;
    },
    addEventListener: (_name: string, listener: () => void) => {
      preferenceChanged = listener;
    },
    removeEventListener: vi.fn(),
  }));
});
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it('expires one confirmed cue and suppresses same-revision replay or restore', () => {
  const { result, rerender } = renderHook(({ current }) => useConfirmedCombatMotion(current), {
    initialProps: { current: frame },
  });
  expect(result.current).toEqual([]);
  rerender({ current: damaged(5, 8) });
  expect(result.current.map((effect) => effect.kind)).toEqual(['damage']);
  act(() => vi.advanceTimersByTime(280));
  expect(result.current).toEqual([]);
  rerender({ current: damaged(5, 8) });
  expect(result.current).toEqual([]);
  rerender({ current: { ...damaged(6, 12), scopeKey: 'campaign:timeline-2:encounter' } });
  expect(result.current).toEqual([]);
});

it('honors changed reduced-motion preference without replaying accumulated effects', () => {
  const { result, rerender } = renderHook(({ current }) => useConfirmedCombatMotion(current), {
    initialProps: { current: frame },
  });
  rerender({ current: damaged(5, 8) });
  expect(result.current).toHaveLength(1);
  act(() => {
    reduced = true;
    preferenceChanged?.();
  });
  expect(result.current).toEqual([]);
  rerender({ current: damaged(6, 4) });
  expect(result.current).toEqual([]);
  act(() => {
    reduced = false;
    preferenceChanged?.();
  });
  expect(result.current).toEqual([]);
  rerender({ current: damaged(7, 2) });
  expect(result.current).toHaveLength(1);
});

it('does not forget an observed revision after a stale projection arrives', () => {
  const { result, rerender } = renderHook(({ current }) => useConfirmedCombatMotion(current), {
    initialProps: { current: frame },
  });
  rerender({ current: damaged(5, 8) });
  expect(result.current).toHaveLength(1);
  rerender({ current: frame });
  expect(result.current).toEqual([]);
  rerender({ current: damaged(5, 8) });
  expect(result.current).toEqual([]);
  rerender({ current: damaged(6, 4) });
  expect(result.current).toHaveLength(1);
});

it('consumes unfocused updates quietly and does not play them on focus return', () => {
  const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(true);
  const { result, rerender } = renderHook(({ current }) => useConfirmedCombatMotion(current), {
    initialProps: { current: frame },
  });
  act(() => {
    focus.mockReturnValue(false);
    window.dispatchEvent(new Event('blur'));
  });
  rerender({ current: damaged(5, 8) });
  expect(result.current).toEqual([]);
  act(() => {
    focus.mockReturnValue(true);
    window.dispatchEvent(new Event('focus'));
  });
  expect(result.current).toEqual([]);
  rerender({ current: damaged(6, 4) });
  expect(result.current).toHaveLength(1);
});
