import { act, renderHook } from '@testing-library/react';
import type { UIEvent } from 'react';
import { describe, expect, it } from 'vitest';
import { useStickyScroll } from '../useStickyScroll';

function makeScrollEvent(scrollTop: number, scrollHeight: number, clientHeight: number) {
  return {
    currentTarget: {
      scrollTop,
      scrollHeight,
      clientHeight,
    } as unknown as HTMLElement,
  } as unknown as UIEvent<HTMLElement>;
}

describe('useStickyScroll', () => {
  it('shouldScroll is true initially', () => {
    const { result } = renderHook(() => useStickyScroll(100));
    expect(result.current.shouldScroll).toBe(true);
  });

  it('shouldScroll becomes false when user scrolls up past threshold', () => {
    const { result } = renderHook(() => useStickyScroll(100));
    act(() => {
      // distanceFromBottom = 1000 - 400 - 0 = 600px > 100
      result.current.onScroll(makeScrollEvent(0, 1000, 400));
    });
    expect(result.current.shouldScroll).toBe(false);
  });

  it('shouldScroll becomes true again when user scrolls back near bottom', () => {
    const { result } = renderHook(() => useStickyScroll(100));
    act(() => {
      result.current.onScroll(makeScrollEvent(0, 1000, 400));
    });
    expect(result.current.shouldScroll).toBe(false);
    act(() => {
      // distanceFromBottom = 1000 - 400 - 560 = 40px < 100
      result.current.onScroll(makeScrollEvent(560, 1000, 400));
    });
    expect(result.current.shouldScroll).toBe(true);
  });

  it('threshold is configurable', () => {
    const { result } = renderHook(() => useStickyScroll(50));
    act(() => {
      // distanceFromBottom = 1000 - 400 - 540 = 60px > 50
      result.current.onScroll(makeScrollEvent(540, 1000, 400));
    });
    expect(result.current.shouldScroll).toBe(false);
  });
});
