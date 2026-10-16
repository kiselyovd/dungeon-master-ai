import { renderHook } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { useFirstEntry } from './useFirstEntry';

it('resolves saved guest routing after hydration once and respects later manual navigation', () => {
  const guest = vi.fn();
  const { rerender } = renderHook(({ hydrated, entry }) => useFirstEntry(hydrated, entry, guest), {
    initialProps: { hydrated: false, entry: 'guest' as 'guest' | 'adventure' | null },
  });
  expect(guest).not.toHaveBeenCalled();
  rerender({ hydrated: true, entry: 'guest' });
  expect(guest).toHaveBeenCalledOnce();
  rerender({ hydrated: true, entry: 'adventure' });
  rerender({ hydrated: true, entry: 'guest' });
  expect(guest).toHaveBeenCalledOnce();
});
