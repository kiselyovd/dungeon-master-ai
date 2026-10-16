import { LazyStore } from '@tauri-apps/plugin-store';
import { afterEach, expect, it } from 'vitest';
import { create } from 'zustand';
import { persistStorage } from './persistStorage';
import { createSettingsSlice, type SettingsSlice } from './settings';

const prefs = new LazyStore('settings.json');
afterEach(async () => {
  await prefs.delete('reading_font_size');
});

it('bounds reading size independently of model and campaign configuration', () => {
  const store = create<SettingsSlice>()(createSettingsSlice);
  const original = store.getState().settings.providers;
  expect(store.getState().settings.readingFontSize).toBe(17);
  for (const size of [16, 17, 18]) {
    store.getState().settings.setReadingFontSize(size);
    expect(store.getState().settings.readingFontSize).toBe(size);
  }
  for (const value of [Number.NaN, -3, 19, 1.7]) {
    store.getState().settings.setReadingFontSize(value);
    expect(store.getState().settings.readingFontSize).toBe(17);
  }
  expect(store.getState().settings.providers).toBe(original);
});

it('round-trips local reading size and ignores unsupported legacy values', async () => {
  await persistStorage.setItem('prefs', {
    state: { settings: { readingFontSize: 18 } },
    version: 0,
  });
  expect(await prefs.get('reading_font_size')).toBe(18);
  expect((await persistStorage.getItem('prefs'))?.state.settings?.readingFontSize).toBe(18);
  await prefs.set('reading_font_size', 400);
  expect((await persistStorage.getItem('prefs'))?.state.settings?.readingFontSize).toBeUndefined();
});
