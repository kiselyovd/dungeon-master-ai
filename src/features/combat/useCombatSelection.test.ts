import { act, renderHook } from '@testing-library/react';
import { expect, it } from 'vitest';
import type { CombatToken } from './model/types';
import { useCombatSelection } from './useCombatSelection';

it('retains a public selection through revisions but clears removed actors and restored timelines', () => {
  const tokens = [{ id: 'one' }, { id: 'two' }] as CombatToken[];
  const { result, rerender } = renderHook(
    ({ scope, actors }) => useCombatSelection(scope, actors),
    { initialProps: { scope: 'encounter:timeline-1', actors: tokens } },
  );
  act(() => result.current.select('two'));
  rerender({ scope: 'encounter:timeline-1', actors: [...tokens] });
  expect(result.current.selectedId).toBe('two');
  act(() => result.current.select('missing'));
  expect(result.current.selectedId).toBe('two');
  rerender({ scope: 'encounter:timeline-1', actors: [tokens[0] as CombatToken] });
  expect(result.current.selectedId).toBeNull();
  rerender({ scope: 'encounter:timeline-2', actors: tokens });
  rerender({ scope: 'encounter:timeline-1', actors: tokens });
  expect(result.current.selectedId).toBeNull();
});
