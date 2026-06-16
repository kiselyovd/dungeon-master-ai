import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useHydrated } from '../useHydrated';

describe('useHydrated', () => {
  it('eventually reports true once persist hydration completes', async () => {
    const { result } = renderHook(() => useHydrated());
    await waitFor(() => expect(result.current).toBe(true));
  });
});
