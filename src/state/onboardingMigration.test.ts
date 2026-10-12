import { LazyStore } from '@tauri-apps/plugin-store';
import { afterEach, expect, it } from 'vitest';
import { persistStorage } from './persistStorage';

const prefs = new LazyStore('settings.json');
afterEach(async () => {
  for (const key of ['onboarding_completed', 'onboarding_entry', 'onboarding_version'])
    await prefs.delete(key);
});
it('migrates legacy completion while preserving a saved guest entry', async () => {
  await prefs.set('onboarding_completed', true);
  const old = await persistStorage.getItem('prefs');
  expect(old?.state.onboarding).toMatchObject({ completed: true, version: 2, entry: 'adventure' });
  await prefs.set('onboarding_completed', false);
  await prefs.set('onboarding_entry', 'guest');
  const partial = await persistStorage.getItem('prefs');
  expect(partial?.state.onboarding).toMatchObject({ completed: false, version: 2, entry: 'guest' });
  await persistStorage.setItem('prefs', {
    version: 0,
    state: { onboarding: { completed: true, version: 2, entry: 'guest' } },
  });
  expect((await persistStorage.getItem('prefs'))?.state.onboarding?.entry).toBe('guest');
});
