import * as v from 'valibot';
import { expect, it } from 'vitest';
import { CharCreationDraftSchema } from '../charCreationSchema';
import { persistStorage } from '../persistStorage';
import { useStore } from '../useStore';

it('roundtrips the real partialized draft and canonical PC defense through storage', async () => {
  const choice = { schema_version: 1 as const, armor_id: null, shield: false };
  useStore.getState().charCreation.resetDraft();
  useStore.getState().charCreation.setDraftField('defenseChoice', choice);
  useStore.getState().pc.replaceFromDraft({ defense: choice, baseSpeedFt: 30 });
  const partialize = useStore.persist.getOptions().partialize;
  expect(partialize).toBeDefined();
  const state = partialize?.(useStore.getState());
  expect(v.safeParse(CharCreationDraftSchema, state?.charCreation).success).toBe(true);
  expect(state?.charCreation?.defenseChoice).toEqual(choice);
  await persistStorage.setItem('defense-roundtrip', { state: state ?? {}, version: 0 });
  const restored = await persistStorage.getItem('defense-roundtrip');
  expect(restored?.state.charCreation?.defenseChoice).toEqual(choice);
  expect(restored?.state.pc?.defense).toEqual(choice);
  expect(restored?.state.pc?.baseSpeedFt).toBe(30);
});
