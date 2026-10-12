import { LazyStore } from '@tauri-apps/plugin-store';
import { afterEach, expect, it, vi } from 'vitest';
import { campaignVisits, recordCampaignVisit } from './campaignVisits';
import { persistStorage } from './persistStorage';

const id = '62ba8579-904b-4460-a68a-df6268ed404a';
afterEach(async () => {
  vi.useRealTimers();
  await new LazyStore('settings.json').delete('campaign_visits');
});
it('retains only bounded valid device navigation metadata', () => {
  const entries = Object.fromEntries(
    Array.from({ length: 60 }, (_, i) => [
      `62ba8579-904b-4460-a68a-${i.toString(16).padStart(12, '0')}`,
      new Date(Date.UTC(2026, 0, i + 1)).toISOString(),
    ]),
  );
  expect(Object.keys(campaignVisits(entries))).toHaveLength(50);
  expect(campaignVisits({ 'bad id': '2026-01-01', [id]: 'bad timestamp' })).toEqual({});
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-10-12T00:00:00Z'));
  expect(recordCampaignVisit({}, id)[id]).toBe('2026-10-12T00:00:00.000Z');
});
it('round-trips navigation history independently of gameplay and provider credentials', async () => {
  const visits = { [id]: '2026-10-12T00:00:00.000Z' };
  await persistStorage.setItem('prefs', {
    version: 0,
    state: { settings: { campaignVisits: visits } },
  });
  expect((await persistStorage.getItem('prefs'))?.state.settings?.campaignVisits).toEqual(visits);
});
